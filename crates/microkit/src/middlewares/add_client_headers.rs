use poem::{http::HeaderValue, Endpoint, Middleware, Request, Result};

use crate::middlewares::CurrentServiceName;

/// Client-side middleware that attaches service-identifying headers to outgoing
/// gRPC requests.
///
/// Two headers are injected:
///
/// | Header | Source | Purpose |
/// |---|---|---|
/// | `x-micro-service` | Extracted from the request URI path (second-to-last segment) | Identifies the **target** service being called |
/// | `x-micro-from-service` | Read from [`CurrentServiceName`] request data (set by the server-side `SetCurrentService` middleware) | Identifies the **calling** service |
///
/// This middleware is typically not used directly — it is registered automatically
/// by the code generator via
/// [`client_middleware("gear_microkit::middlewares::AddClientHeaders")`](https://docs.rs/poem-grpc-build).
pub struct AddClientHeaders;

impl<E: Endpoint> Middleware<E> for AddClientHeaders {
    type Output = AddClientHeadersEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        AddClientHeadersEndpoint { inner: ep }
    }
}

/// The endpoint wrapper produced by [`AddClientHeaders`].
///
/// See [`AddClientHeaders`] for details on the headers that are injected.
pub struct AddClientHeadersEndpoint<E> {
    inner: E,
}

impl<E: Endpoint> Endpoint for AddClientHeadersEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        // x-micro-service
        if let Some(service) = req.uri().path().split('/').rev().nth(1) {
            if let Ok(service) = HeaderValue::from_str(service) {
                req.headers_mut().insert("x-micro-service", service);
            }
        }

        // x-micro-from-service
        if let Some(service_name) = req
            .data::<CurrentServiceName>()
            .and_then(|service_name| service_name.0.parse().ok())
        {
            req.headers_mut()
                .insert("x-micro-from-service", service_name);
        }

        self.inner.call(req).await
    }
}
