use std::hash::{Hash, Hasher};
use std::time::Duration;
use std::collections::HashMap;
use lazy_static::lazy_static;
use governor::{Quota, RateLimiter as GovernorRateLimiter};
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;
use nonzero_ext::nonzero;
use std::sync::Mutex;

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
    pub fn apply(&self, path: &str) -> Option<T> {
        if !Self::check_for_rate_limiting(path) {
           return Some(self.response.clone())
        }
        None
    }

    fn check_for_rate_limiting(path: &str) -> bool {
        let rate_limiter_key  = path.to_string();

        if RATELIMITER_CACHE.lock().unwrap().get("default" ).is_none() {
            let default_quota = Quota::with_period(Duration::from_secs(1))
                .unwrap();
            RATELIMITER_CACHE.lock().unwrap().insert("default".parse().unwrap(), GovernorRateLimiter::keyed(default_quota));
        }

        if RATELIMITER_CACHE.lock().unwrap().get(&rate_limiter_key).is_none() {
            RATELIMITER_CACHE.lock().unwrap().get("default").unwrap().check_key(&rate_limiter_key) == Ok(())
        } else {
            RATELIMITER_CACHE.lock().unwrap().get(&rate_limiter_key).unwrap().check_key(&rate_limiter_key) == Ok(())
        }
    }

    pub fn create_rate_limiter(time_window: Duration, threshold_count: i32, burst_percentage: i32, path: &str)
    {
        let rate_limiter_key = path.to_string();
        if RATELIMITER_CACHE.lock().unwrap().get(&rate_limiter_key ).is_none() {
            let time_window_in_secs = time_window.as_secs();
            //let max_burst = nonzero!(((burst_percentage as u32)/100)*(*&threshold_count as u32));
            let quota = Quota::with_period(Duration::from_secs(time_window_in_secs/(threshold_count as u64)))
                .unwrap();
            //.allow_burst(max_burst);
            RATELIMITER_CACHE.lock().unwrap().insert(rate_limiter_key , GovernorRateLimiter::keyed(quota));
        }
    }
}

lazy_static! {
    static ref RATELIMITER_CACHE:
    Mutex<HashMap<String, GovernorRateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>>> = Mutex::new(HashMap::new());
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

