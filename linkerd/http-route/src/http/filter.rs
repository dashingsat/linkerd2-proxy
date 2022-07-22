pub mod inject_failure;
pub mod modify_header;
pub mod redirect;
pub mod rate_limiter;

pub use self::{
    inject_failure::{Distribution, FailureResponse, InjectFailure},
    modify_header::ModifyHeader,
    redirect::{InvalidRedirect, RedirectRequest, Redirection},
    rate_limiter::{RateLimiter,FailureResponse as RateLimiterFailureResponse, Configuration as RateLimitConfiguration, create_rate_limiter},
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum ModifyPath {
    ReplaceFullPath(String),
    ReplacePrefixMatch(String),
}
