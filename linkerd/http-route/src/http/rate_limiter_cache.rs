use std::collections::HashMap;
use std::time::Duration;
use lazy_static::lazy_static;
use governor::{Quota, RateLimiter};
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;
use nonzero_ext::nonzero;
use std::sync::Mutex;

lazy_static! {
    static ref RATELIMITER_CACHE:
    Mutex<HashMap<&'static str, RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>>> = Mutex::new(HashMap::new());
}

fn create_rate_limiter(time_window: Duration, threshold_count: i32, burst_percentage: i32)
                       -> RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>
{
    let time_window_in_secs = time_window.as_secs();
    //let max_burst = nonzero!(((burst_percentage as u32)/100)*(*&threshold_count as u32));
    let quota = Quota::with_period(Duration::from_secs(time_window_in_secs/(threshold_count as u64)))
        .unwrap();
        //.allow_burst(max_burst);
    RateLimiter::keyed(quota)
}

fn get_rate_limiter<'a>(service: &'static str, path: &str, time_window: Duration, threshold_count: i32, burst_percentage: i32)
                                -> Option<&'a RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>> {
    if RATELIMITER_CACHE.lock().unwrap().get(&service).is_none() {
        let rate_limiter = create_rate_limiter(time_window, threshold_count, burst_percentage);
        RATELIMITER_CACHE.lock().unwrap().insert(&service, rate_limiter);
    }
    RATELIMITER_CACHE.lock().unwrap().get(&service)
}


