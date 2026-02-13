use std::io;

use opentelemetry::{global, trace::TracerProvider as _};
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider};
use poem::{
    endpoint::BoxEndpoint,
    listener::TcpListener,
    middleware::{AddData, OpenTelemetryMetrics, OpenTelemetryTracing, TokioMetrics},
    EndpointExt, IntoEndpoint, Middleware, Response, Server,
};
use poem_grpc::{RouteGrpc, Service};

use crate::middlewares::{RequestDurationMiddleware, SetCurrentService};

/// A gRPC server with production-ready defaults.
///
/// `GrpcServer` wraps a [`poem_grpc::RouteGrpc`] router and applies a standard
/// middleware stack when started:
///
/// | Middleware | Purpose |
/// |---|---|
/// | [`AddData`] | Injects the OpenTelemetry [`Tracer`](opentelemetry_sdk::trace::Tracer) into request data |
/// | [`Compression`] | Transparent response compression |
/// | [`OpenTelemetryTracing`] | Distributed tracing for incoming requests |
/// | [`OpenTelemetryMetrics`] | Request-level OpenTelemetry metrics |
/// | `SetCurrentService` | Extracts the target service name from the URI and stores it as request data |
/// | [`TokioMetrics`] | Tokio runtime metrics (opt-in via `GEAR_ENABLE_TOKIO_METRICS=1`) |
/// | `RequestDurationMiddleware` | Per-method Prometheus histogram (`micro_request_duration_seconds`) |
///
/// The server listens on the address specified by the `MICRO_SERVER_ADDRESS` environment
/// variable, falling back to `0.0.0.0:8080` if unset.
///
/// # Examples
///
/// Start a server with a single gRPC service:
///
/// ```rust,no_run
/// use gear_microkit::GrpcServer;
/// # struct MyService;
/// # impl poem::IntoEndpoint for MyService {
/// #     type Endpoint = poem::endpoint::BoxEndpoint<'static, poem::Response>;
/// #     fn into_endpoint(self) -> Self::Endpoint { todo!() }
/// # }
/// # impl poem_grpc::Service for MyService {
/// #     const NAME: &'static str = "my.Service";
/// # }
///
/// #[tokio::main]
/// async fn main() -> std::io::Result<()> {
///     GrpcServer::new()
///         .add_service(MyService)
///         .start()
///         .await
/// }
/// ```
#[derive(Default)]
pub struct GrpcServer {
    router: RouteGrpc,
}

impl GrpcServer {
    /// Creates a new `GrpcServer` with an empty router.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gear_microkit::GrpcServer;
    ///
    /// let server = GrpcServer::new();
    /// ```
    pub fn new() -> Self {
        Default::default()
    }

    /// Registers a gRPC service with the server.
    ///
    /// Multiple services can be registered by chaining calls. Each service must
    /// implement both [`IntoEndpoint`] (producing a boxed endpoint) and
    /// [`Service`](poem_grpc::Service).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use gear_microkit::GrpcServer;
    /// # struct UserService;
    /// # struct OrderService;
    /// # impl poem::IntoEndpoint for UserService {
    /// #     type Endpoint = poem::endpoint::BoxEndpoint<'static, poem::Response>;
    /// #     fn into_endpoint(self) -> Self::Endpoint { todo!() }
    /// # }
    /// # impl poem_grpc::Service for UserService {
    /// #     const NAME: &'static str = "user.UserService";
    /// # }
    /// # impl poem::IntoEndpoint for OrderService {
    /// #     type Endpoint = poem::endpoint::BoxEndpoint<'static, poem::Response>;
    /// #     fn into_endpoint(self) -> Self::Endpoint { todo!() }
    /// # }
    /// # impl poem_grpc::Service for OrderService {
    /// #     const NAME: &'static str = "order.OrderService";
    /// # }
    /// let server = GrpcServer::new()
    ///     .add_service(UserService)
    ///     .add_service(OrderService);
    /// ```
    pub fn add_service<S>(mut self, service: S) -> Self
    where
        S: IntoEndpoint<Endpoint = BoxEndpoint<'static, Response>> + Service,
    {
        self.router = self.router.add_service(service);
        self
    }

    /// Starts the server with an additional user-supplied middleware applied
    /// **outermost** (i.e. it wraps all built-in middleware).
    ///
    /// This is useful when you need to add custom authentication, rate-limiting,
    /// or other cross-cutting concerns on top of the default middleware stack.
    ///
    /// Pass `()` as the middleware to use only the built-in stack (equivalent to
    /// calling [`start`](Self::start)).
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if the TCP listener fails to bind or the server
    /// encounters a fatal runtime error.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use gear_microkit::GrpcServer;
    /// use poem::middleware::SetHeader;
    ///
    /// #[tokio::main]
    /// async fn main() -> std::io::Result<()> {
    ///     GrpcServer::new()
    ///         .start_with_middleware(SetHeader::new().appending("x-powered-by", "gear"))
    ///         .await
    /// }
    /// ```
    pub async fn start_with_middleware<T>(self, middleware: T) -> io::Result<()>
    where
        T: Middleware<BoxEndpoint<'static, Response>> + 'static,
    {
        global::set_text_map_propagator(TraceContextPropagator::new());
        let tracer_provider = SdkTracerProvider::builder()
            .with_batch_exporter(
                opentelemetry_otlp::SpanExporter::builder()
                    .with_tonic()
                    .build()
                    .expect("Trace exporter should initialize."),
            )
            .build();
        let tracer = tracer_provider.tracer("gear-rs");
        let app = self
            .router
            .with(
                AddData::new(tracer.clone())
                    .combine(OpenTelemetryTracing::new(tracer))
                    .combine(OpenTelemetryMetrics::new())
                    .combine(SetCurrentService)
                    .combine_if(
                        std::env::var("GEAR_ENABLE_TOKIO_METRICS").as_deref() == Ok("1"),
                        TokioMetrics::new(),
                    )
                    .combine(RequestDurationMiddleware::new()),
            )
            .boxed();
        let app = app.with(middleware);

        let grpc_server = Server::new(TcpListener::bind(
            std::env::var("MICRO_SERVER_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
        ))
        .http2_max_concurrent_streams(None)
        .http2_max_header_list_size(16384 * 64)
        .run(app);
        tokio::try_join!(grpc_server).map(|_| ())
    }

    /// Starts the server with only the built-in middleware stack.
    ///
    /// This is a convenience shorthand for
    /// [`start_with_middleware(())`](Self::start_with_middleware).
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if the TCP listener fails to bind or the server
    /// encounters a fatal runtime error.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use gear_microkit::GrpcServer;
    ///
    /// #[tokio::main]
    /// async fn main() -> std::io::Result<()> {
    ///     GrpcServer::new()
    ///         .start()
    ///         .await
    /// }
    /// ```
    pub async fn start(self) -> io::Result<()> {
        self.start_with_middleware(()).await
    }
}
