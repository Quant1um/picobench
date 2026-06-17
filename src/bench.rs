use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

/// This is where the benchmarks are stored.
static BENCHMARKS: Mutex<Vec<Benchmark>> = Mutex::new(Vec::new());

/// A benchmark that can be run
#[derive(Clone)]
pub struct Benchmark {
    /// The name of the benchmark
    pub name: String,

    /// The callback to run the benchmark, which takes the number of iterations to run and returns the time taken in seconds
    pub callback: Arc<dyn Fn(u64) -> f64 + Send + Sync>,

    /// The minimum amount of time to collect samples for.
    pub sample_time: Duration,

    /// The minimum number of iterations to run the benchmark for, regardless of the time taken.
    pub sample_size: u64,

    /// The amount of time to warm up the benchmark before collecting samples.
    pub warmup_time: Duration,

    /// Confidence level (p-value) threshold for reporting regressions.
    pub confidence_level: f64,
}

impl Benchmark {
    /// Registers a new benchmark to be run later when calling [`picobench::run()`](crate::run).
    pub fn register(self) {
        BENCHMARKS.lock().unwrap().push(self);
    }

    /// Returns a list of all registered benchmarks.
    pub fn list() -> Vec<Benchmark> {
        BENCHMARKS.lock().unwrap().clone()
    }

    /// Creates a new benchmark with the given name and callback.
    pub fn new(name: impl Into<String>, callback: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            name: name.into(),
            callback: Arc::new(move |iters| {
                let start = fastant::Instant::now();
                for _ in 0..iters {
                    callback();
                }
                as_millis(start.elapsed())
            }),
            warmup_time: Duration::from_millis(100),
            sample_time: Duration::from_millis(100),
            sample_size: 100,
            confidence_level: 0.0005,
        }
    }

    /// Run the benchmark with the given number of iterations and stopping criteria.
    pub fn run(&self) -> (Vec<f64>, u64) {
        /// Sample time is chosen in such a way to minimize the overhead of the benchmark itself and to minimize the effects of limited clock precision,
        /// while being short enough to get enough samples in a reasonable time.
        const SAMPLE_TIME: Duration = Duration::from_micros(500);

        let mut total_time = 0.0;
        let mut total_iters = 0u64;
        let mut iters_per_sample = 1u64;

        // warmup phase & compute the number of iterations per sample
        while total_time < as_millis(self.warmup_time) {
            let time = (self.callback)(iters_per_sample);

            match total_iters.checked_add(iters_per_sample) {
                Some(iters) => {
                    total_time += time;
                    total_iters = iters;
                    iters_per_sample = iters_per_sample.saturating_mul(2);
                }
                None => {
                    break; // we overflowed, just stop
                }
            };
        }

        // sampling phase, use the number of iterations per sample computed in the warmup phase
        let mut results = Vec::new();
        let mut sample_time = 0.0;
        let mut sample_iters = 0u64;
        let mut sample_size = 0u64;
        let iters_per_sample =
            (as_millis(SAMPLE_TIME) / (total_time / total_iters as f64)).ceil() as u64;

        while sample_time < as_millis(self.sample_time) || sample_size < self.sample_size {
            let time = (self.callback)(iters_per_sample);

            match sample_iters.checked_add(iters_per_sample) {
                Some(iters) => {
                    results.push(time / iters_per_sample as f64);
                    sample_iters = iters;
                    sample_time += time;
                    sample_size += 1;
                }
                None => break, // we overflowed, just stop
            };
        }

        (results, sample_iters)
    }
}

fn as_millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

/// Our `#[bench]` proc-macro internally forwards to this declarative macro.
#[doc(hidden)]
#[macro_export]
#[cfg(feature = "macros")]
macro_rules! define_benchmark {
    (
        #[picobench::bench()]
        $(#[$($attr:meta)*])*
        $(pub)? fn $name:ident() $(-> $output:ty)? $body:block
    ) => {
        $(#[$($attr)*])*
        #[doc(hidden)]
        mod $name {
            use super::*;

            #[$crate::ctor]
            unsafe fn register() {
                $crate::Benchmark::new(
                    module_path!(),
                    #[inline(always)]
                    || $body,
                )
                .register();
            }
        }
    };
}
