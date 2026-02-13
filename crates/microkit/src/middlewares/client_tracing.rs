use opentelemetry::{
    global,
    trace::{FutureExt, Span, SpanKind, TraceContextExt, Tracer as _},
    Context, KeyValue,
};
use opentelemetry_http::HeaderInjector;
use opentelemetry_sdk::trace::Tracer;
use opentelemetry_semantic_conventions::trace;
use poem::{Endpoint, Middleware, Request, Result};

/// Client-side middleware that creates an OpenTelemetry span for each outgoing
/// gRPC request and propagates the trace context via HTTP headers.
///
/// When a [`Tracer`] is present in the request data (injected by the server's
/// `AddData` middleware), this middleware will:
///
/// 1. Start a new span named `"grpc request"` with [`SpanKind::Client`].
/// 2. Record the full request URI as the [`URL_FULL`](trace::URL_FULL) attribute.
/// 3. Inject the current trace context into the outgoing request headers using
///    the globally configured [`TextMapPropagator`](opentelemetry::propagation::TextMapPropagator)
///    (typically W3C `traceparent` / `tracestate`).
/// 4. Execute the inner endpoint within the span's context so that downstream
///    calls are correctly parented.
///
/// If no `Tracer` is found in request data, the request is forwarded as-is
/// without any tracing overhead.
///
/// This middleware is typically not used directly — it is registered automatically
/// by the code generator via
/// [`client_middleware("gear_microkit::middlewares::ClientTracing")`](https://docs.rs/poem-grpc-build).
pub struct ClientTracing;

impl<E: Endpoint> Middleware<E> for ClientTracing {
    type Output = ClientTracingEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        ClientTracingEndpoint { inner: ep }
    }
}

/// The endpoint wrapper produced by [`ClientTracing`].
///
/// See [`ClientTracing`] for details on the tracing behavior.
pub struct ClientTracingEndpoint<E> {
    inner: E,
}

impl<E: Endpoint> Endpoint for ClientTracingEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        match req.data::<Tracer>() {
            Some(tracer) => {
                let mut span = tracer
                    .span_builder("grpc request")
                    .with_kind(SpanKind::Client)
                    .start(tracer);
                span.set_attribute(KeyValue::new(trace::URL_FULL, req.uri().path().to_string()));

                let cx = Context::current_with_span(span);
                global::get_text_map_propagator(|propagator| {
                    propagator.inject_context(&cx, &mut HeaderInjector(req.headers_mut()))
                });

                self.inner.call(req).with_context(cx).await
            }
            None => self.inner.call(req).await,
        }
    }
}
