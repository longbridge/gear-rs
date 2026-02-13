#![allow(clippy::declare_interior_mutable_const)] // https://github.com/tokio-rs/tokio/issues/4872#issuecomment-1197753788

use poem::{Endpoint, Middleware, Request, Result};

pub(crate) struct SetCurrentService;

impl<E: Endpoint> Middleware<E> for SetCurrentService {
    type Output = SetCurrentServiceEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        SetCurrentServiceEndpoint { inner: ep }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CurrentServiceName(pub(crate) String);

pub(crate) struct SetCurrentServiceEndpoint<E> {
    inner: E,
}

impl<E: Endpoint> Endpoint for SetCurrentServiceEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        // x-micro-service
        if let Some(service) = req.uri().path().split('/').rev().nth(1) {
            req.set_data(CurrentServiceName(service.to_string()));
        }
        self.inner.call(req).await
    }
}
