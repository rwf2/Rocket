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

            let h3listener = crate::listener::quic::QuicListener::bind(addr, tls.clone())
                .map_err(|e| ErrorKind::Bind(Some(endpoint.clone()), Box::new(e)))
                .await?;

            let rocket = self.into_orbit(vec![h3listener.endpoint()?, endpoint]);
            let rocket = orbit_callback(rocket).await?;

            let http12 = tokio::task::spawn(rocket.clone().serve12(listener));
            let http3 = tokio::task::spawn(rocket.clone().serve3(h3listener));
            let (r1, r2) = tokio::join!(http12, http3);
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
            .half_close(true)
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
    async fn serve3(self: Arc<Self>, listener: crate::listener::quic::QuicListener) -> Result<()> {
        let rocket = self.clone();
        let listener = Arc::new(listener);
        while let Some(Some(accept)) = listener.accept().race(rocket.shutdown()).await.left() {
            let (listener, rocket) = (listener.clone(), rocket.clone());
            spawn_inspect(|e: &io::Error| log_server_error(e), async move {
                let mut stream = listener.connect(accept).race_io(rocket.shutdown()).await?;
                while let Some(mut conn) = stream.accept().race_io(rocket.shutdown()).await? {
                    let rocket = rocket.clone();
                    spawn_inspect(|e: &io::Error| log_server_error(e), async move {
                        let meta = ConnectionMeta::new(conn.endpoint(), None, None);
                        let rx = conn.rx.cancellable(rocket.shutdown.clone());
                        let response = rocket.clone()
                            .service(conn.parts, rx, None, meta)
                            .map_err(io::Error::other)
                            .race_io(rocket.shutdown.mercy.clone())
                            .await?;

                        let grace = rocket.shutdown.grace.clone();
                        match conn.tx.send_response(response).race(grace).await.left() {
                            Some(result) => result,
                            None => Ok(conn.tx.cancel()),
                        }
                    });
                }

                Ok(())
            });
        }

        Ok(())
    }
}
