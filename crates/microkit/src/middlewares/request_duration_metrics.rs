use std::{sync::Arc, time::Instant};

use poem::{Endpoint, Middleware, Request, Result};
use prometheus::{histogram_opts, register_histogram_vec, HistogramVec};

pub(crate) struct RequestDurationMiddleware {
    histogram: Arc<HistogramVec>,
}

impl Default for RequestDurationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestDurationMiddleware {
    pub(crate) fn new() -> Self {
        let opts = histogram_opts!(
            "micro_request_duration_seconds",
            "rpc method request time in seconds"
        );
        let histogram = register_histogram_vec!(opts, &["method", "status", "caller"])
            .expect("failed to create request_duration_seconds histogram");
        Self {
            histogram: Arc::new(histogram),
        }
    }
}

impl<E: Endpoint> Middleware<E> for RequestDurationMiddleware {
    type Output = RequestDurationEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        RequestDurationEndpoint {
            inner: ep,
            histogram: self.histogram.clone(),
        }
    }
}

pub(crate) struct RequestDurationEndpoint<E> {
    inner: E,
    histogram: Arc<HistogramVec>,
}

impl<E: Endpoint> Endpoint for RequestDurationEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let start = Instant::now();
        let method = req.uri().path().to_string();

        let caller = req
            .header("x-micro-from-service")
            .unwrap_or_default()
            .to_string();
        let response = self.inner.call(req).await;
        let duration = start.elapsed().as_secs_f64();
        let status = if response.is_ok() { "success" } else { "fail" };

        self.histogram
            .with_label_values(&[&*method, status, &*caller])
            .observe(duration);

        response
    }
}
