use std::hash::{Hash, Hasher};
use std::time::Duration;
use std::collections::HashMap;
use lazy_static::lazy_static;
use governor::{Quota, RateLimiter as GovernorRateLimiter};
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;
use nonzero_ext::nonzero;
use std::sync::Mutex;
use http::StatusCode;

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
        None
    }
    fn check_for_rate_limiting_inner(path: &str) -> bool {
        let rate_limiter_key = path.to_string();

        if RATELIMITER_CACHE.lock().unwrap().get("default").is_none() {
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
}

pub fn default_err_message() -> StatusCode {
    StatusCode::TOO_MANY_REQUESTS
}

pub fn check_for_rate_limiting(path: &str) -> bool {
    let rate_limiter_key = path.to_string();

    if RATELIMITER_CACHE.lock().unwrap().get("default").is_none() {
        let default_quota = Quota::with_period(Duration::from_secs(5))
            .unwrap();
        RATELIMITER_CACHE.lock().unwrap().insert("default".parse().unwrap(), GovernorRateLimiter::keyed(default_quota));
    }

    if RATELIMITER_CACHE.lock().unwrap().get(&rate_limiter_key).is_none() {
        RATELIMITER_CACHE.lock().unwrap().get("default").unwrap().check_key(&rate_limiter_key) == Ok(())
    } else {
        RATELIMITER_CACHE.lock().unwrap().get(&rate_limiter_key).unwrap().check_key(&rate_limiter_key) == Ok(())
    }
}

pub fn create_rate_limiter(time_window: Duration, threshold_count: u32, burst_percentage: f64, path: &str)
{
    let rate_limiter_key = path.to_string();
    if RATELIMITER_CACHE.lock().unwrap().get(&rate_limiter_key).is_none() {
        let time_window_in_secs = time_window.as_secs();
        //let max_burst = nonzero!(((burst_percentage as u32)/100)*(*&threshold_count as u32));
        if (threshold_count > *&time_window_in_secs as u32) {
            let quota = Quota::with_period(Duration::from_millis(time_window_in_secs*1000 / (threshold_count as u64)))
                .unwrap();
            //.allow_burst(max_burst);
            RATELIMITER_CACHE.lock().unwrap().insert(rate_limiter_key, GovernorRateLimiter::keyed(quota));
        } else {
            let quota = Quota::with_period(Duration::from_secs(time_window_in_secs / (threshold_count as u64)))
                .unwrap();
            //.allow_burst(max_burst);
            RATELIMITER_CACHE.lock().unwrap().insert(rate_limiter_key, GovernorRateLimiter::keyed(quota));
        }

    }
}

lazy_static! {
    static ref RATELIMITER_CACHE:
    Mutex<HashMap<String, GovernorRateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>>> = Mutex::new(HashMap::new());
}
