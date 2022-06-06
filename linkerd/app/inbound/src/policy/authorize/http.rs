use crate::metrics::authz::HttpAuthzMetrics;

use super::super::{AllowPolicy, Permit};
use futures::{future, TryFutureExt};
use linkerd_app_core::{
    svc::{self, ServiceExt},
    tls,
    transport::{ClientAddr, Remote},
    Error,
};
use std::task;

/// A middleware that enforces policy on each HTTP request.
///
/// This enforcement is done lazily on each request so that policy updates are honored as the
/// connection progresses.
///
/// The inner service is created for each request, so it's expected that this is combined with
/// caching.
#[derive(Clone, Debug)]
pub struct NewAuthorizeHttp<N> {
    metrics: HttpAuthzMetrics,
    inner: N,
}

#[derive(Clone, Debug)]
pub struct AuthorizeHttp<T, N> {
    target: T,
    client_addr: Remote<ClientAddr>,
    tls: tls::ConditionalServerTls,
    policy: AllowPolicy,
    metrics: HttpAuthzMetrics,
    inner: N,
}

// === impl NewAuthorizeHttp ===

impl<N> NewAuthorizeHttp<N> {
    pub fn layer(metrics: HttpAuthzMetrics) -> impl svc::layer::Layer<N, Service = Self> + Clone {
        svc::layer::mk(move |inner| Self {
            metrics: metrics.clone(),
            inner,
        })
    }
}

impl<T, N> svc::NewService<T> for NewAuthorizeHttp<N>
where
    T: svc::Param<AllowPolicy>
        + svc::Param<Remote<ClientAddr>>
        + svc::Param<tls::ConditionalServerTls>,
    N: Clone,
{
    type Service = AuthorizeHttp<T, N>;

    fn new_service(&self, target: T) -> Self::Service {
        let client_addr = target.param();
        let tls = target.param();
        let policy = target.param();
        AuthorizeHttp {
            target,
            client_addr,
            tls,
            policy,
            metrics: self.metrics.clone(),
            inner: self.inner.clone(),
        }
    }
}

// === impl AuthorizeHttp ===

impl<Req, T, N, S> svc::Service<Req> for AuthorizeHttp<T, N>
where
    T: Clone,
    N: svc::NewService<(Permit, T), Service = S>,
    S: svc::Service<Req>,
    S::Error: Into<Error>,
{
    type Response = S::Response;
    type Error = Error;
    type Future = future::Either<
        future::ErrInto<svc::stack::Oneshot<S, Req>, Error>,
        future::Ready<Result<Self::Response, Error>>,
    >;

    #[inline]
    fn poll_ready(&mut self, _: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Req) -> Self::Future {
        tracing::trace!(policy = ?self.policy, "Authorizing request");
        match self.policy.check_authorized(self.client_addr, &self.tls) {
            Ok(permit) => {
                tracing::debug!(
                    ?permit,
                    tls = ?self.tls,
                    client = %self.client_addr,
                    "Request authorized",
                );
                self.metrics.allow(&permit, self.tls.clone());
                let svc = self.inner.new_service((permit, self.target.clone()));
                future::Either::Left(svc.oneshot(req).err_into::<Error>())
            }
            Err(e) => {
                tracing::info!(
                    server = %format_args!("{}:{}", self.policy.server_label().kind, self.policy.server_label().name),
                    tls = ?self.tls,
                    client = %self.client_addr,
                    "Request denied",
                );
                self.metrics.deny(&self.policy, self.tls.clone());
                future::Either::Right(future::err(e.into()))
            }
        }
    }
}

/*
use linkerd_http_route::{
    filter::{InvalidRedirect, ModifyRequestHeader, Redirection},
    service::Routes,
    ApplyRoute, HttpRouteMatch,
};

#[derive(Debug, thiserror::Error)]
pub enum RouteError {
    #[error("invalid redirect: {0}")]
    InvalidRedirect(#[from] InvalidRedirect),

    #[error("request redirected")]
    Redirect(Redirection),
}


// === impl HttpConfig ===

impl Routes for HttpConfig {
    type Route = RoutePolicy;
    type Error = RouteError;

    fn find<B>(&self, req: &http::Request<B>) -> Option<(HttpRouteMatch, &RoutePolicy)> {
        linkerd_http_route::find(&*self.routes, req)
    }

    fn apply<B>(&self, route: &Self::Route, req: &mut http::Request<B>) -> Result<(), Self::Error> {
        todo!()
    }
}

// === impl RoutePolicy ===

impl ApplyRoute for RoutePolicy {
    type Error = RouteError;

    fn apply_route<B>(
        &self,
        rm: HttpRouteMatch,
        req: &mut http::Request<B>,
    ) -> Result<(), RouteError> {
        // TODO use request extensions to find client information.
        for authz in &*self.authorizations {
            let _ = authz;
        }

        for filter in &self.filters {
            match filter {
                RouteFilter::RequestHeaders(rh) => {
                    rh.apply(req.headers_mut());
                }
                RouteFilter::Redirect(redir) => {
                    let redirection = redir.apply(req.uri(), &rm)?;
                    return Err(RouteError::Redirect(redirection));
                }
                RouteFilter::Unknown => {
                    // XXX should we throw an error? log a warning?
                }
            }
        }

        req.extensions_mut().insert(self.labels.clone());

        Ok(())
    }
}
*/
