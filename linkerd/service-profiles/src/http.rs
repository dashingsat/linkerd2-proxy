mod proxy;
mod service;

use regex::Regex;
use std::{
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
    sync::Arc,
    time::Duration,
};
use tower::retry::budget::Budget;

pub use self::{proxy::NewProxyRouter, service::NewServiceRouter};

#[derive(Clone, Debug, Default, PartialEq, Eq,  Hash)]
pub struct Route {
    labels: Labels,
    response_classes: ResponseClasses,
    retries: Option<Retries>,
    timeout: Option<Duration>,
    rate_limit_config: Option<RateLimitingConfig>
}

#[derive(Clone, Debug)]
pub enum RequestMatch {
    All(Vec<RequestMatch>),
    Any(Vec<RequestMatch>),
    Not(Box<RequestMatch>),
    Path(Box<Regex>),
    Method(http::Method),
}

#[derive(Clone, Debug)]
pub struct ResponseClass {
    is_failure: bool,
    match_: ResponseMatch,
}

#[derive(Clone, Debug)]
pub struct RateLimitingConfig {
    pub request_threshold_count: u32,
    pub time_window: ::core::option::Option<Duration>,
    pub burst_percentage: f64
}

#[derive(Clone, Default)]
pub struct ResponseClasses(Arc<Vec<ResponseClass>>);

#[derive(Clone, Debug)]
pub enum ResponseMatch {
    All(Vec<ResponseMatch>),
    Any(Vec<ResponseMatch>),
    Not(Box<ResponseMatch>),
    Status {
        min: http::StatusCode,
        max: http::StatusCode,
    },
}

#[derive(Clone, Debug)]
pub struct Retries {
    budget: Arc<Budget>,
}

#[derive(Clone, Default)]
struct Labels(Arc<std::collections::BTreeMap<String, String>>);

fn route_for_request<'r, B>(
    http_routes: &'r [(RequestMatch, Route)],
    request: &http::Request<B>,
) -> Option<&'r Route> {
    for (request_match, route) in http_routes {
        if request_match.is_match(request) {
            return Some(route);
        }
    }
    None
}

// === impl Route ===

impl Route {
    pub fn new<I>(label_iter: I, response_classes: Vec<ResponseClass>) -> Self
    where
        I: Iterator<Item = (String, String)>,
    {
        let labels = Labels(Arc::new(label_iter.collect()));

        Self {
            labels,
            response_classes: ResponseClasses(response_classes.into()),
            retries: None,
            timeout: None,
            rate_limit_config: None
        }
    }

    pub fn labels(&self) -> &Arc<std::collections::BTreeMap<String, String>> {
        &self.labels.0
    }

    pub fn response_classes(&self) -> &ResponseClasses {
        &self.response_classes
    }

    pub fn retries(&self) -> Option<&Retries> {
        self.retries.as_ref()
    }

    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    pub fn rate_limit_config(&self) -> Option<&RateLimitingConfig> {
        self.rate_limit_config.as_ref()
    }

    pub fn set_retries(&mut self, budget: Arc<Budget>) {
        self.retries = Some(Retries { budget });
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Some(timeout);
    }

    pub fn set_rate_limit(&mut self, rate_limit_config: RateLimitingConfig) {
        self.rate_limit_config = Some(rate_limit_config)
    }
}

// === impl RequestMatch ===

impl RequestMatch {
    fn is_match<B>(&self, req: &http::Request<B>) -> bool {
        match self {
            RequestMatch::Method(ref method) => req.method() == *method,
            RequestMatch::Path(ref re) => re.is_match(req.uri().path()),
            RequestMatch::Not(ref m) => !m.is_match(req),
            RequestMatch::All(ref ms) => ms.iter().all(|m| m.is_match(req)),
            RequestMatch::Any(ref ms) => ms.iter().any(|m| m.is_match(req)),
        }
    }
}

// === impl ResponseClass ===

impl ResponseClass {
    pub fn new(is_failure: bool, match_: ResponseMatch) -> Self {
        Self { is_failure, match_ }
    }

    pub fn is_failure(&self) -> bool {
        self.is_failure
    }

    pub fn is_match<B>(&self, req: &http::Response<B>) -> bool {
        self.match_.is_match(req)
    }
}

// === impl ResponseClasses ===

impl Deref for ResponseClasses {
    type Target = [ResponseClass];

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl PartialEq for ResponseClasses {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ResponseClasses {}

impl PartialEq<Self> for RateLimitingConfig {
    fn eq(&self, other: &Self) -> bool {
        self.time_window == other.time_window && self.request_threshold_count == other.request_threshold_count
    }
}

impl Eq for RateLimitingConfig {}

impl Hash for ResponseClasses {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(Arc::as_ref(&self.0) as *const _ as usize);
    }
}

impl Hash for RateLimitingConfig{
    fn hash<H: Hasher>(&self, state: &mut H) {
       self.request_threshold_count.hash(state)
    }
}

impl fmt::Debug for ResponseClasses {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// === impl ResponseMatch ===

impl ResponseMatch {
    fn is_match<B>(&self, req: &http::Response<B>) -> bool {
        match self {
            ResponseMatch::Status { ref min, ref max } => {
                *min <= req.status() && req.status() <= *max
            }
            ResponseMatch::Not(ref m) => !m.is_match(req),
            ResponseMatch::All(ref ms) => ms.iter().all(|m| m.is_match(req)),
            ResponseMatch::Any(ref ms) => ms.iter().any(|m| m.is_match(req)),
        }
    }
}

// === impl Retries ===

impl Retries {
    pub fn budget(&self) -> &Arc<Budget> {
        &self.budget
    }
}

impl PartialEq for Retries {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.budget, &other.budget)
    }
}

impl Eq for Retries {}

impl Hash for Retries {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(Arc::as_ref(&self.budget) as *const _ as usize);
    }
}

// === impl Labels ===

impl PartialEq for Labels {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Labels {}

impl Hash for Labels {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(Arc::as_ref(&self.0) as *const _ as usize);
    }
}

impl fmt::Debug for Labels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
