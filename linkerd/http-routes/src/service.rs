use super::InboundRoutes;
use crate::filter::{Filter, InvalidRedirect, Redirection};
use futures::TryFutureExt;
use linkerd_stack::Service;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::watch;

#[derive(Clone, Debug)]
pub struct InboundService<S> {
    watch: watch::Receiver<InboundRoutes>,
    inner: S,
}

#[derive(Debug, thiserror::Error)]
pub enum Error<E> {
    #[error("no matching route for the request")]
    NoRoute,

    #[error("request redirected")]
    Redirect(Redirection),

    #[error("invalid request redirect")]
    InvalidRedirect(#[from] InvalidRedirect),

    #[error("{0}")]
    Inner(#[source] E),
}

impl<S> InboundService<S> {
    pub fn new(watch: watch::Receiver<InboundRoutes>, inner: S) -> Self {
        Self { watch, inner }
    }
}

impl<B, S> Service<http::Request<B>> for InboundService<S>
where
    S: Service<http::Request<B>>,
    S::Response: Send + 'static,
    S::Error: Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = Error<S::Error>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Error<S::Error>>> + Send>>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Error::Inner)
    }

    #[inline]
    fn call(&mut self, mut req: http::Request<B>) -> Self::Future {
        let routes = self.watch.borrow();
        let (_route, rule, rtmatch) = match routes.find_route(&req) {
            Some(r) => r,
            None => return Box::pin(futures::future::err(Error::NoRoute)),
        };
        // TODO apply route authorization
        for f in &rule.filters {
            match f {
                Filter::ModifyRequestHeader(f) => f.apply(req.headers_mut()),
                Filter::RedirectRequest(f) => {
                    return Box::pin(match f.apply(req.uri(), &rtmatch) {
                        Ok(redirect) => futures::future::err(Error::Redirect(redirect)),
                        Err(invalid) => futures::future::err(invalid.into()),
                    })
                }
            }
        }
        // TODO labels
        Box::pin(self.inner.call(req).map_err(Error::Inner))
    }
}
