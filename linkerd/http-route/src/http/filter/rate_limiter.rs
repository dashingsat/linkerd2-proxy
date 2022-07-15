use std::hash::{Hash, Hasher};
use std::time::Duration;

/// A filter that responds with an error at a predictable rate.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct RateLimiter<T = FailureResponse> {
    pub response: T,
    pub configuration: Configuration,
}

/// An HTTP error response.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FailureResponse {
    pub status: http::StatusCode,
    pub message: std::sync::Arc<str>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Configuration {
    pub threshold: i32,
    pub duration: Duration
}

impl<T: Clone> RateLimiter<T> {
    pub fn apply(&self) -> Option<T> {
        if Self::check_for_rate_limiting(&self.configuration) {
           return Some(self.response.clone())
        }
        None
    }
    fn check_for_rate_limiting(_configuration: &Configuration) -> bool {
      true
    }
}

// impl PartialEq for Configuration {
//     fn eq(&self, other: &Self) -> bool {
//         true
//     }
// }
//
// impl Hash for Configuration {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         todo!()
//     }
// }
//
//
// impl Eq for Configuration {}
//
// impl Hash for Distribution {
//     fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//         self.numerator.hash(state);
//         self.denominator.hash(state);
//     }

