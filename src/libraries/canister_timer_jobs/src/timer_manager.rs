use crate::Environment;
use ic_cdk_timers::TimerId;
use job_macros::job;
use std::any::type_name;
use std::time::Duration;
use types::TimestampMillis;

#[job(attempts = 3, interval = 60)]
fn my_job() {
    println!("This is the job function.");
}

// NOTE: https://nullderef.com/blog/rust-async-sync/#_what_ended_up_working_the_maybe_async_crate
// NOTE: https://www.byronwasti.com/async-func-pointers/
pub struct TimerManager<J, R>
where
    J: Fn() -> R,
    R: 'static,
{
    job_function: J,
    function_name: String, // Store name just in case
    timer_id: Option<TimerId>,
    interval: Duration,
    max_attempts: u32,
    retry_delay_duration: Duration,
    last_run: Option<TimestampMillis>,
}

impl<J, R> std::fmt::Debug for TimerManager<J, R>
where
    J: Fn() -> R,
    R: 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimerManager")
            .field("function_name", &self.function_name)
            .field("timer_id", &self.timer_id)
            .field("interval", &self.interval)
            .field("max_attempts", &self.max_attempts)
            .field("retry_delay_duration", &self.retry_delay_duration)
            .field("last_run", &self.last_run)
            .finish()
    }
}

// Sync version
impl<J> TimerManager<J, Result<(), String>>
where
    J: Fn() -> Result<(), String> + Clone + Sync + 'static,
{
    pub fn start_timer_sync(&mut self, env: &dyn Environment) {
        let interval = self.interval;
        let job_function = self.job_function.clone();
        let max_attempts = self.max_attempts;
        let retry_delay_duration = self.retry_delay_duration;

        self.timer_id = Some(ic_cdk_timers::set_timer_interval(interval, move || {
            run_sync(job_function.clone(), max_attempts, retry_delay_duration)
        }));

        self.last_run = Some(env.now());
    }
}

// Async version
impl<J, R> TimerManager<J, R>
where
    J: Fn() -> R + Clone + 'static + Sync + Send,
    R: std::future::Future<Output = Result<(), String>>,
{
    pub fn start_timer_async(&mut self, env: &dyn Environment) {
        let interval = self.interval;
        let job_function = self.job_function.clone();
        let max_attempts = self.max_attempts;
        let retry_delay_duration = self.retry_delay_duration;

        self.timer_id = Some(ic_cdk_timers::set_timer_interval(interval, move || {
            run_async(job_function.clone(), max_attempts, retry_delay_duration)
        }));

        self.last_run = Some(env.now());
    }
}

impl<J, R> TimerManager<J, R>
where
    J: Fn() -> R,
    R: 'static,
{
    pub fn new(
        job_function: J,
        interval_secs: u64,
        max_attempts: Option<u32>,
        retry_delay_duration: Option<Duration>,
    ) -> Self {
        let function_name = type_name::<J>().to_string();
        // job_function.
        Self {
            job_function,
            function_name,
            timer_id: None,
            interval: Duration::from_secs(interval_secs),
            max_attempts: max_attempts.unwrap_or(1),
            retry_delay_duration: retry_delay_duration.unwrap_or_default(),
            last_run: None,
        }
    }

    pub fn cancel_timer(&mut self) {
        if let Some(timer_id) = self.timer_id.take() {
            ic_cdk_timers::clear_timer(timer_id);
        }
    }

    pub fn get_function_name(&self) -> &str {
        &self.function_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::BuildVersion;
    use utils::env::CanisterEnv;

    // Example functions
    async fn example_async_action() -> Result<(), String> {
        println!("Async Action executed");
        Ok(())
    }

    fn example_sync_action() -> Result<(), String> {
        println!("Sync Action executed");
        Ok(())
    }

    // #[test]
    // fn test_sync_job() {
    //     let env = CanisterEnv::new(true, BuildVersion::min(), "test".to_string());
    //     let mut timer = TimerManager::new(example_sync_action, 2, None, None);
    //     // NOTE: this will not work outside of the canister, but allows to see if the compiler is ok with the code
    //     timer.start_timer_sync(&env);
    //     println!("Timer: {:?}", timer);
    //     timer.cancel_timer();
    // }

    // #[tokio::test]
    // async fn test_async_job() {
    //     let env = CanisterEnv::new(true, BuildVersion::min(), "test".to_string());
    //     let mut timer = TimerManager::new(example_async_action, 2, None, None);
    //     // NOTE: this will not work outside of the canister, but allows to see if the compiler is ok with the code
    //     timer.start_timer_async(&env);
    //     println!("Timer: {:?}", timer);
    //     timer.cancel_timer();
    // }
}

pub async fn retry_with_attempts_sync<F>(max_attempts: u32, _delay_duration: Duration, mut f: F)
where
    F: FnMut() -> Result<(), String>,
{
    for attempt in 1..=max_attempts {
        match f() {
            Ok(_) => {
                break;
            } // If successful, break out of the loop
            Err(err) => {
                error!("Attempt {}: Error - {:?}", attempt, err);
                if attempt == max_attempts {
                    error!(
                        "Failed to execute the action after {} attempts: {:?}",
                        max_attempts, err
                    );
                }
            }
        }
    }
}

// NOTE: Helper function for retry logic with attempts
use tracing::error;
pub async fn retry_with_attempts_async<F, Fut>(
    max_attempts: u32,
    _delay_duration: Duration,
    mut f: F,
) where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    for attempt in 1..=max_attempts {
        match f().await {
            Ok(_) => {
                break;
            } // If successful, break out of the loop
            Err(err) => {
                error!("Attempt {}: Error - {:?}", attempt, err);
                if attempt == max_attempts {
                    error!(
                        "Failed to execute the action after {} attempts: {:?}",
                        max_attempts, err
                    );
                }
            }
        }
    }
}

// NOTE: RUN DIFFERENT TYPES
fn run_sync<F>(func: F, max_attempts: u32, retry_delay_duration: Duration)
where
    F: Fn() -> Result<(), String>,
{
    let _ = retry_with_attempts_sync(max_attempts, retry_delay_duration, func);
}

pub fn run_async<F, Fut>(func: F, max_attempts: u32, retry_delay_duration: Duration)
where
    F: Fn() -> Fut + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + 'static,
{
    ic_cdk::spawn(async move {
        let _ = retry_with_attempts_async(max_attempts, retry_delay_duration, func).await;
    });
}

// #[test]
// fn test() {
//     // Example of synchronous function
//     fn sync_function() {
//         println!("Sync function executed");
//     }

//     // Example of asynchronous function
//     async fn async_function() {
//         println!("Async function executed");
//     }

//     ic_cdk::spawn(async_function());
// }
