use rocket::{get, launch, routes, State};
use rocket_grpc::serve_grpc_default;
use serde::Serialize;
use tonic::{Request, Response, Status};
use tokio_stream::{wrappers::ReceiverStream, Stream};
use std::pin::Pin;

// Include the generated code from the proto file
pub mod greeter {
    tonic::include_proto!("greeter");
}

use greeter::{
    greeter_server::{Greeter, GreeterServer},
    HelloRequest, HelloReply,
};

#[derive(Clone)]
pub struct AppState {
    pub greeting_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

// HTTP Routes
#[get("/")]
fn index() -> &'static str {
    "Hello from Rocket HTTP server! gRPC server is running on port 50051."
}

#[get("/stats")]
fn stats(state: &State<AppState>) -> rocket::serde::json::Json<Stats> {
    let count = state.greeting_count.load(std::sync::atomic::Ordering::Relaxed);
    rocket::serde::json::Json(Stats { greeting_count: count })
}

#[derive(Serialize)]
struct Stats {
    greeting_count: u64,
}

// gRPC Service Implementation
#[derive(Clone)]
pub struct MyGreeter {
    state: AppState,
}

impl MyGreeter {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request: {:?}", request);
        
        // Increment the greeting counter
        self.state.greeting_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        let reply = HelloReply {
            message: format!("Hello {}! (from gRPC)", request.into_inner().name),
        };

        Ok(Response::new(reply))
    }

    type SayHelloStreamStream = Pin<Box<dyn Stream<Item = Result<HelloReply, Status>> + Send>>;

    async fn say_hello_stream(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<Self::SayHelloStreamStream>, Status> {
        println!("Got a streaming request: {:?}", request);
        
        let name = request.into_inner().name;
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        
        // Increment counter for stream request
        self.state.greeting_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
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

#[launch]
fn rocket() -> _ {
    let app_state = AppState {
        greeting_count: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
    };

    let greeter_service = MyGreeter::new(app_state.clone());
    
    rocket::build()
        .manage(app_state)
        .mount("/", routes![index, stats])
        .attach(serve_grpc_default(GreeterServer::new(greeter_service)))
}