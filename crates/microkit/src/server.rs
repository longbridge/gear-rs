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

/// GRPC Server
#[derive(Default)]
pub struct GrpcServer {
    router: RouteGrpc,
}

impl GrpcServer {
    /// Create a GRPC server
    pub fn new() -> Self {
        Default::default()
    }

    /// Add a GRPC service
    pub fn add_service<S>(mut self, service: S) -> Self
    where
        S: IntoEndpoint<Endpoint = BoxEndpoint<'static, Response>> + Service,
    {
        self.router = self.router.add_service(service);
        self
    }

    /// Start the server with the middleware
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

    /// Start the server
    pub async fn start(self) -> io::Result<()> {
        self.start_with_middleware(()).await
    }
}
