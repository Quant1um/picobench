use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

/// This is where the benchmarks are stored.
static BENCHMARKS: Mutex<Vec<Benchmark>> = Mutex::new(Vec::new());

/// A benchmark that can be run
#[derive(Clone)]
pub struct Benchmark {
    /// The path of the benchmark (e.g. "my_module::my_benchmark"), used for display purposes and to group benchmarks together.
    pub path: Arc<str>,

    /// The callback to run the benchmark, which takes the number of iterations to run and returns the time taken in seconds
    pub callback: Arc<dyn Fn(u64) -> f64 + Send + Sync>,

    /// The minimum amount of time to collect samples for.
    pub sample_time: Duration,

    /// The minimum number of iterations to run the benchmark for, regardless of the time taken.
    pub sample_size: u64,

    /// The amount of time to warm up the benchmark before collecting samples.
    pub warmup_time: Duration,
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
    pub fn new(path: impl Into<String>, callback: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            path: path.into().into(),
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
        }
    }

    /// Sets the warmup time for the benchmark.
    ///
    /// This is the amount of time to run the benchmark before collecting samples.
    ///
    /// Warmup is also used to determine the number of iterations to run per sample, so that the benchmark runs for a reasonable amount of time.
    pub fn warmup_time(mut self, warmup_time: Duration) -> Self {
        self.warmup_time = warmup_time;
        self
    }

    /// Sets the sample time for the benchmark.
    ///
    /// This is the minimum amount of time to collect samples for.
    pub fn sample_time(mut self, sample_time: Duration) -> Self {
        self.sample_time = sample_time;
        self
    }

    /// Sets the sample size for the benchmark.
    ///
    /// This is the minimum number of iterations to run the benchmark for, regardless of the time taken.
    pub fn sample_size(mut self, sample_size: u64) -> Self {
        self.sample_size = sample_size;
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into().into();
        self
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
        #[picobench::bench($($property:ident = $value:expr),* $(,)?)]
        $(#[$($attr:meta)*])*
        $vis:vis fn $name:ident() $(-> $output:ty)? $body:block
    ) => {
        $(#[$($attr)*])*
        #[inline(always)]
        $vis fn $name() $(-> $output)? {
            #[$crate::ctor]
            #[doc(hidden)]
            unsafe fn __picobench() {
                $crate::Benchmark::new(
                    concat!(module_path!(), "::", stringify!($name)),
                    #[inline(always)]
                    || $name(),
                )
                $(
                    .$property($value)
                )*
                .register();
            }

            $body
        }
    };
}
