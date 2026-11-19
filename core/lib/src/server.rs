use std::io;
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::server::conn::auto::Builder;
use futures::{Future, TryFutureExt};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{Ignite, Orbit, Request, Rocket};
use crate::request::ConnectionMeta;
use crate::erased::{ErasedRequest, ErasedResponse, ErasedIoHandler};
use crate::listener::{Listener, Connection, BouncedExt, CancellableExt};
use crate::error::log_server_error;
use crate::data::{IoStream, RawStream};
use crate::util::{spawn_inspect, FutureExt, ReaderStream};
use crate::http::Status;
use crate::trace::{Trace, TraceAll};

type Result<T, E = crate::Error> = std::result::Result<T, E>;

impl Rocket<Orbit> {
    #[tracing::instrument("request", skip_all, fields(
        method = %parts.method,
        uri = %parts.uri,
        autohandled
    ))]
    async fn service<T: for<'a> Into<RawStream<'a>>>(
        self: Arc<Self>,
        parts: http::request::Parts,
        stream: T,
        upgrade: Option<hyper::upgrade::OnUpgrade>,
        connection: ConnectionMeta,
    ) -> Result<hyper::Response<ReaderStream<ErasedResponse>>, http::Error> {
        connection.trace_debug();
        let request = ErasedRequest::new(self, parts, |rocket, parts| {
            Request::from_hyp(rocket, parts, connection).unwrap_or_else(|e| e)
        });

        span_debug!("request headers" => request.inner().headers().iter().trace_all_debug());
        let mut response = request.into_response(
            stream,
            |rocket, request, data| Box::pin(rocket.preprocess(request, data)),
            |token, rocket, request, data| Box::pin(async move {
                if !request.errors.is_empty() {
                    return rocket.dispatch_error(Status::BadRequest, request).await;
                }

                rocket.dispatch(token, request, data).await
            })
        ).await;

        // TODO: Should upgrades be handled in dispatch?
        response.inner().trace_info();
        span_debug!("response headers" => response.inner().headers().iter().trace_all_debug());
        let io_handler = response.make_io_handler(Rocket::extract_io_handler);
        if let (Some((proto, handler)), Some(upgrade)) = (io_handler, upgrade) {
            let upgrade = upgrade.map_ok(IoStream::from).map_err(io::Error::other);
            tokio::task::spawn(io_handler_task(proto, upgrade, handler));
        }

        let mut builder = hyper::Response::builder();
        builder = builder.status(response.inner().status().code);
        for header in response.inner().headers().iter() {
            builder = builder.header(header.name().as_str(), header.value());
        }

        let chunk_size = response.inner().body().max_chunk_size();
        builder.body(ReaderStream::with_capacity(response, chunk_size))
    }

    pub(crate) fn alt_svc(&self) -> Option<&'static str> {
        cfg!(feature = "http3-preview").then(|| {
            static ALT_SVC: state::InitCell<Option<String>> = state::InitCell::new();

            ALT_SVC.get_or_init(|| {
                let addr = self.endpoints().find_map(|v| v.quic())?;
                Some(format!("h3=\":{}\"", addr.port()))
            }).as_deref()
        })?
    }
}

#[tracing::instrument("upgrade", skip_all, fields(protocol = proto))]
async fn io_handler_task<S>(proto: String, stream: S, mut handler: ErasedIoHandler)
    where S: Future<Output = io::Result<IoStream>>
{
    let stream = match stream.await {
        Ok(stream) => stream,
        Err(e) => return warn!(error = %e, "i/o upgrade failed"),
    };

    debug!("i/o upgrade succeeded");
    if let Err(e) = handler.take().io(stream).await {
        match e.kind() {
            io::ErrorKind::BrokenPipe => warn!("i/o handler closed"),
            _ => warn!(error = %e, "i/o handler terminated unsuccessfully"),
        }
    }
}

impl Rocket<Ignite> {
    pub(crate) async fn listen_and_serve<L, R>(
        self,
        listener: L,
        orbit_callback: impl FnOnce(Rocket<Orbit>) -> R,
    ) -> Result<Arc<Rocket<Orbit>>>
        where L: Listener + 'static,
              R: Future<Output = Result<Arc<Rocket<Orbit>>>>
    {
        let endpoint = listener.endpoint()?;

        #[cfg(feature = "http3-preview")]
        if let (Some(addr), Some(tls)) = (endpoint.tcp(), endpoint.tls_config()) {
            use crate::error::ErrorKind;

            let quic = crate::listener::quic::bind(addr, tls)
                .map_err(|e| ErrorKind::Bind(Some(endpoint.clone()), Box::new(e)))
                .await?;

            let quic_endpoint = crate::listener::Endpoint::Quic(quic.local_addr()?).with_tls(tls);
            let rocket = self.into_orbit(vec![quic_endpoint, endpoint]);
            let rocket = orbit_callback(rocket).await?;

            let http12 = tokio::task::spawn(rocket.clone().serve12(listener));
            let http3 = tokio::task::spawn(rocket.clone().serve3(quic));
            let (r1, r2) = futures::join!(http12, http3);
            r1.map_err(|e| ErrorKind::Liftoff(Err(rocket.clone()), e))??;
            r2.map_err(|e| ErrorKind::Liftoff(Err(rocket.clone()), e))??;
            return Ok(rocket);
        }

        if cfg!(feature = "http3-preview") {
            warn!("HTTP/3 cannot start without a valid TCP + TLS configuration.\n\
                Falling back to HTTP/1 + HTTP/2 server.");
        }

        let rocket = self.into_orbit(vec![endpoint]);
        let rocket = orbit_callback(rocket).await?;
        rocket.clone().serve12(listener).await?;
        Ok(rocket)
    }
}

impl Rocket<Orbit> {
    pub(crate) async fn serve12<L>(self: Arc<Self>, listener: L) -> Result<()>
        where L: Listener + 'static,
              L::Connection: AsyncRead + AsyncWrite
    {
        let mut builder = Builder::new(TokioExecutor::new());
        let keep_alive = Duration::from_secs(self.config.keep_alive.into());
        builder.http1()
            .timer(TokioTimer::new())
            .keep_alive(keep_alive > Duration::ZERO)
            .preserve_header_case(true)
            .header_read_timeout(Duration::from_secs(15));

        #[cfg(feature = "http2")] {
            builder.http2().timer(TokioTimer::new());
            if keep_alive > Duration::ZERO {
                builder.http2()
                    .timer(TokioTimer::new())
                    .keep_alive_interval(keep_alive / 4)
                    .keep_alive_timeout(keep_alive);
            }
        }

        let (listener, server) = (Arc::new(listener.bounced()), Arc::new(builder));
        while let Some(accept) = listener.accept().race(self.shutdown()).await.left().transpose()? {
            let (listener, rocket, server) = (listener.clone(), self.clone(), server.clone());
            spawn_inspect(|e| log_server_error(&**e), async move {
                let conn = listener.connect(accept).race_io(rocket.shutdown()).await?;
                let meta = ConnectionMeta::new(
                    conn.endpoint(),
                    conn.certificates(),
                    conn.server_name()
                );
                let service = service_fn(|mut req| {
                    let upgrade = hyper::upgrade::on(&mut req);
                    let (parts, incoming) = req.into_parts();
                    rocket.clone().service(parts, incoming, Some(upgrade), meta.clone())
                });

                let io = TokioIo::new(conn.cancellable(rocket.shutdown.clone()));
                let mut server = pin!(server.serve_connection_with_upgrades(io, service));
                match server.as_mut().race(rocket.shutdown()).await.left() {
                    Some(result) => result,
                    None => {
                        server.as_mut().graceful_shutdown();
                        server.await
                    },
                }
            });
        }

        Ok(())
    }

    #[cfg(feature = "http3-preview")]
    async fn serve3(self: Arc<Self>, endpoint: quinn::Endpoint) -> Result<()> {
        use std::future::IntoFuture;
        use either::Either;
        use crate::listener::{Endpoint, quic::{self, QuicRx}};

        while let Some(Some(incoming)) = endpoint.accept().race(self.shutdown()).await.left() {
            let rocket = self.clone();
            spawn_inspect(|e: &io::Error| log_server_error(e), async move {
                let conn = incoming.into_future()
                    .map_err(io::Error::other)
                    .race_io(rocket.shutdown())
                    .await?;

                let remote = Endpoint::Quic(conn.remote_address()).assume_tls();
                let (certs, sni) = (quic::certificates(&conn), quic::server_name(&conn));
                let meta = ConnectionMeta::new(Ok(remote), certs, sni.as_deref());
                let mut h3 = quic::Connection::new(h3_quinn::Connection::new(conn))
                    .map_err(io::Error::other)
                    .await?;

                loop {
                    let resolver = match h3.accept().race(rocket.shutdown()).await {
                        Either::Left(Ok(Some(resolver))) => resolver,
                        Either::Left(Ok(None)) => break,
                        Either::Left(Err(e)) if e.is_h3_no_error() => break,
                        Either::Left(Err(e)) => return Err(io::Error::other(e)),
                        Either::Right(_) => {
                            // Send `GOAWAY` so the client makes no new requests, then
                            // let in-flight requests finish within the grace period.
                            h3.shutdown(0).await.map_err(io::Error::other)?;
                            let _ = h3.accept().race(rocket.shutdown.grace.clone()).await;
                            break;
                        }
                    };

                    let (rocket, meta) = (rocket.clone(), meta.clone());
                    spawn_inspect(|e: &io::Error| log_server_error(e), async move {
                        let (req, stream) = resolver.resolve_request()
                            .map_err(io::Error::other)
                            .race_io(rocket.shutdown())
                            .await?;

                        let (mut tx, rx) = stream.split();
                        let rx = QuicRx(rx).cancellable(rocket.shutdown.clone());
                        let response = rocket.clone()
                            .service(req.into_parts().0, rx, None, meta)
                            .map_err(io::Error::other)
                            .race_io(rocket.shutdown.mercy.clone())
                            .await?;

                        let grace = rocket.shutdown.grace.clone();
                        match quic::send_response(&mut tx, response).race(grace).await.left() {
                            Some(result) => result,
                            None => Ok(tx.stop_stream(h3::error::Code::H3_REQUEST_CANCELLED)),
                        }
                    });
                }

                Ok(())
            });
        }

        // Refuse new connections and give existing ones a chance to close.
        endpoint.set_server_config(None);
        endpoint.wait_idle().race(self.shutdown.mercy.clone()).await;
        Ok(())
    }
}
