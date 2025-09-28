# Rocket gRPC Support

[![crates.io](https://img.shields.io/crates/v/rocket_grpc.svg)](https://crates.io/crates/rocket_grpc)
[![Rocket Homepage](https://img.shields.io/badge/web-rocket.rs-red.svg?style=flat&label=https&colorB=d33847)](https://rocket.rs)

This crate provides gRPC server support for [Rocket] applications, enabling you to serve both HTTP and gRPC traffic from the same Rocket application. This makes it ideal for microservices that need to expose both REST APIs and gRPC services.

## Features

- **Dual Protocol Support**: Serve both HTTP and gRPC from the same Rocket application
- **Shared State**: Access Rocket's managed state from within gRPC service implementations  
- **Easy Integration**: Uses Rocket's fairing system for seamless integration
- **Streaming Support**: Full support for unary and streaming gRPC methods
- **Tonic Integration**: Built on top of the popular [tonic] gRPC library

## Quick Start

Add the following to your `Cargo.toml`:

```toml
[dependencies]
rocket = { path = "../../core/lib", features = ["json"] }
rocket_grpc = { path = "../../contrib/grpc" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
tonic = "0.14.2"
prost = "0.14.1"
tonic-prost = "0.14.2"
tokio-stream = "0.1"
serde = { version = "1.0", features = ["derive"] }

[build-dependencies]
tonic-prost-build = "0.14.2"
```

Create a `build.rs` file to generate Rust code from your proto definitions:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::compile_protos("proto/greeter.proto")?;
    Ok(())
}
```

Then use the `serve_grpc` fairing to add gRPC support to your Rocket application:

```rust
use rocket::{get, launch, routes};
use rocket_grpc::serve_grpc_default;
use tonic::{Request, Response, Status};

// Include generated proto code
pub mod greeter {
    tonic::include_proto!("greeter");
}

use greeter::{
    greeter_server::{Greeter, GreeterServer},
    HelloRequest, HelloReply,
};

#[derive(Clone)]
pub struct MyGreeter;

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request: {:?}", request);
        
        let reply = HelloReply {
            message: format!("Hello {}! (from gRPC)", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

#[get("/")]
fn index() -> &'static str {
    "Hello from Rocket HTTP server! gRPC server is running on port 50051."
}

#[launch]
fn rocket() -> rocket::Rocket<rocket::Build> {
    let greeter_service = MyGreeter;
    
    rocket::build()
        .mount("/", routes![index])
        .attach(serve_grpc_default(GreeterServer::new(greeter_service)))
}
```

## API Reference

### Core Functions

- **`serve_grpc(service, port)`** - Create a gRPC fairing for the specified service and port
- **`serve_grpc_default(service)`** - Create a gRPC fairing using default port 50051

### Service Utilities

The `service` module provides utilities for integrating gRPC services with Rocket:

- **`StateAwareService<S, T>`** - Wrapper that provides access to Rocket's managed state from gRPC services
  - `new(service, state)` - Create with state access
  - `without_state(service)` - Create without state access
  - `state()` - Get reference to the state
  - `service()` - Get reference to the wrapped service

- **`metadata_to_headers(metadata)`** - Convert gRPC metadata to Rocket-style header map (HashMap<String, String>)

- **`grpc_service!` macro** - Simplifies creating state-aware gRPC services with automatic Clone derive and state management methods

## Shared State Example

You can share state between HTTP and gRPC handlers:

```rust
use rocket::{State, get, launch, routes};
use rocket_grpc::serve_grpc_default;
use serde::Serialize;
use tonic::{Request, Response, Status};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

// Include generated proto code
pub mod greeter {
    tonic::include_proto!("greeter");
}

use greeter::{
    greeter_server::{Greeter, GreeterServer},
    HelloRequest, HelloReply,
};

#[derive(Clone)]
pub struct AppState {
    pub greeting_count: Arc<AtomicU64>,
}

#[derive(Clone)]
pub struct MyGreeter {
    state: AppState,
}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        // Increment the greeting counter
        self.state.greeting_count.fetch_add(1, Ordering::Relaxed);
        
        let reply = HelloReply {
            message: format!("Hello {}! (from gRPC)", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

#[derive(Serialize)]
struct Stats {
    greeting_count: u64,
}

#[get("/")]
fn index() -> &'static str {
    "Hello from Rocket HTTP server! gRPC server is running on port 50051."
}

#[get("/stats")]
fn stats(state: &State<AppState>) -> rocket::serde::json::Json<Stats> {
    let count = state.greeting_count.load(Ordering::Relaxed);
    rocket::serde::json::Json(Stats { greeting_count: count })
}

#[launch]
fn rocket() -> rocket::Rocket<rocket::Build> {
    let app_state = AppState {
        greeting_count: Arc::new(AtomicU64::new(0)),
    };

    let greeter_service = MyGreeter { state: app_state.clone() };
    
    rocket::build()
        .manage(app_state)
        .mount("/", routes![index, stats])
        .attach(serve_grpc_default(GreeterServer::new(greeter_service)))
}
```

## Streaming gRPC Example

The framework supports streaming gRPC methods. Here's how to implement server streaming:

```rust
use tokio_stream::{wrappers::ReceiverStream, Stream};
use std::pin::Pin;

#[tonic::async_trait]
impl Greeter for MyGreeter {
    // ... other methods ...

    type SayHelloStreamStream = Pin<Box<dyn Stream<Item = Result<HelloReply, Status>> + Send>>;

    async fn say_hello_stream(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<Self::SayHelloStreamStream>, Status> {
        let name = request.into_inner().name;
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        
        // Increment counter for stream request
        self.state.greeting_count.fetch_add(1, Ordering::Relaxed);
        
        tokio::spawn(async move {
            for i in 0..5 {
                let reply = HelloReply {
                    message: format!("Hello {} (stream message {})!", name, i + 1),
                };
                
                if tx.send(Ok(reply)).await.is_err() {
                    break;
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::SayHelloStreamStream
        ))
    }
}
```

Your proto file should define the streaming method:

```protobuf
syntax = "proto3";

package greeter;

service Greeter {
  rpc SayHello (HelloRequest) returns (HelloReply) {}
  rpc SayHelloStream (HelloRequest) returns (stream HelloReply) {}
}

message HelloRequest {
  string name = 1;
}

message HelloReply {
  string message = 1;
}
```

## Examples

See the [examples/grpc](../../examples/grpc/) directory for a complete working example that includes both unary and streaming methods.

## Testing

The crate includes a comprehensive test suite covering all major functionality:

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

The test suite covers:

- **StateAwareService**: Testing state access, cloning, and service wrapping
- **Metadata Conversion**: Converting gRPC metadata to header maps
- **grpc_service! Macro**: Testing macro-generated services with various state types
- **Edge Cases**: Complex state types, optional state handling, and service lifecycle

All tests are located in `tests/service.rs` and provide examples of how to use the various utilities.

## Requirements

- Rust 1.75+
- Rocket 0.6.0-dev  
- Tokio runtime
- Protocol Buffers compiler (`protoc`) for building examples

## License

`rocket_grpc` is licensed under either of the following, at your option:

 * Apache License, Version 2.0, ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT License ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

[Rocket]: https://rocket.rs
[tonic]: https://github.com/hyperium/tonic