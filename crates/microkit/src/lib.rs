//! # gear-microkit
//!
//! A microservice infrastructure toolkit providing batteries-included gRPC server
//! bootstrapping, middleware, and request metadata extraction built on top of
//! [poem](https://docs.rs/poem) and [poem-grpc](https://docs.rs/poem-grpc).
//!
//! ## Key Components
//!
//! - [`GrpcServer`] — A gRPC server with built-in OpenTelemetry tracing, Prometheus
//!   metrics, compression, and other production-ready middleware.
//! - [`RequestExt`] — An extension trait for gRPC requests that extracts common
//!   business fields (e.g. `member_id`, `app_id`, `platform`) from request metadata.
//! - [`middlewares`] — Poem middleware used by codegen-generated gRPC clients.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use gear_microkit::GrpcServer;
//!
//! #[tokio::main]
//! async fn main() -> std::io::Result<()> {
//!     GrpcServer::new()
//!         // .add_service(my_grpc_service)
//!         .start()
//!         .await
//! }
//! ```

/// Client-side middleware intended to be injected into codegen-generated gRPC clients.
///
/// The following middleware are publicly re-exported:
///
/// - [`middlewares::AddClientHeaders`] — Attaches `x-micro-service` and
///   `x-micro-from-service` headers to outgoing requests.
/// - [`middlewares::ClientTracing`] — Creates an OpenTelemetry client span and
///   propagates trace context on outgoing requests.
pub mod middlewares;

mod request_ext;
mod server;

pub use request_ext::RequestExt;
pub use server::GrpcServer;
