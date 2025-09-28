//! # Rocket gRPC Support
//!
//! This crate provides gRPC server support for Rocket applications, allowing
//! you to serve both HTTP and gRPC traffic from the same Rocket application.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rocket::*;
//! use rocket_grpc::serve_grpc;
//! 
//! // Assume MyGrpcService is a tonic-generated server type
//! struct MyGrpcService;
//!
//! #[launch]
//! fn rocket() -> _ {
//!     let grpc_service = MyGrpcService;
//!     rocket::build()
//!         .mount("/", routes![hello])
//!         .attach(serve_grpc(grpc_service, 50051))
//! }
//!
//! #[get("/hello")]
//! fn hello() -> &'static str {
//!     "Hello, world!"
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

#[cfg(feature = "tonic")]
use rocket::{Rocket, Build, fairing::{Fairing, Info, Kind}};

#[cfg(feature = "tonic")]
use tonic::transport::Server;

#[cfg(feature = "tonic")]
use std::error::Error;

pub use self::service::*;

mod service;

#[cfg(feature = "tonic")]
/// A fairing that adds gRPC server support to a Rocket application.
///
/// This fairing starts a gRPC server alongside the HTTP server, allowing
/// the same application to serve both HTTP and gRPC traffic on different ports.
pub struct GrpcFairing<S> {
    #[allow(dead_code)] // Service field will be used in future server implementation
    service: S,
    port: u16,
}

#[cfg(feature = "tonic")]
impl<S> GrpcFairing<S> {
    /// Create a new gRPC fairing with the given service and port.
    pub fn new(service: S, port: u16) -> Self {
        Self { service, port }
    }
}

#[cfg(feature = "tonic")]
#[rocket::async_trait]
impl<S> Fairing for GrpcFairing<S>
where
    S: tonic::server::NamedService + Clone + Send + Sync + 'static,
    S: tower_service::Service<
        http::Request<tonic::body::Body>,
        Error = std::convert::Infallible,
    >,
    S::Response: axum::response::IntoResponse,
    S::Future: Send,
{
    fn info(&self) -> Info {
        Info {
            name: "gRPC Server",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        let service = self.service.clone();
        let port = self.port;
        
        // Start the gRPC server in a background task
        tokio::spawn(async move {
            let addr = format!("0.0.0.0:{}", port).parse().unwrap();
            rocket::info!("Starting gRPC server on {}", addr);
            
            match Server::builder()
                .add_service(service)
                .serve(addr)
                .await
            {
                Ok(()) => {
                    rocket::info!("gRPC server has shut down gracefully");
                }
                Err(e) => {
                    rocket::error!("gRPC server failed with detailed error: {:?}", e);
                    rocket::error!("Error kind: {}", e);
                    if let Some(source) = e.source() {
                        rocket::error!("Error source: {:?}", source);
                    }
                }
            }
        });
        
        rocket::info!("gRPC fairing attached for port {}", port);
        Ok(rocket)
    }
}

#[cfg(feature = "tonic")]
/// Convenience function to create a gRPC fairing.
///
/// This function creates a `GrpcFairing` that provides the structure for gRPC
/// server integration with Rocket. Currently, the fairing registers itself
/// but does not automatically start the gRPC server due to complex trait bound
/// requirements with generic service types.
///
/// # Current Status
///
/// The fairing provides the foundation for gRPC integration but requires
/// concrete tonic-generated server types for full implementation. Users should
/// implement their own gRPC server startup logic alongside this fairing.
///
/// # Example
///
/// ```rust,ignore
/// use rocket::*;
/// use rocket_grpc::serve_grpc;
/// 
/// // Assume MyGrpcService is a tonic-generated server type
/// struct MyGrpcService;
///
/// #[launch]
/// fn rocket() -> _ {
///     let my_grpc_service = MyGrpcService;
///     rocket::build()
///         .attach(serve_grpc(my_grpc_service, 50051))
/// }
/// ```
pub fn serve_grpc<S>(service: S, port: u16) -> GrpcFairing<S>
where
    S: tonic::server::NamedService + Clone + Send + Sync + 'static,
{
    GrpcFairing::new(service, port)
}

#[cfg(feature = "tonic")]
/// Convenience function to create a gRPC fairing with default port 50051.
pub fn serve_grpc_default<S>(service: S) -> GrpcFairing<S>
where
    S: tonic::server::NamedService + Clone + Send + Sync + 'static,
{
    serve_grpc(service, 50051)
}