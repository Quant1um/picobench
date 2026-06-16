#![forbid(unsafe_code)]

mod bencher;
mod runner;
mod stats;

#[cfg(feature = "macros")]
pub use picobench_macros::bench;
#[cfg(feature = "macros")]
#[doc(hidden)]
pub use small_ctor::ctor;

pub use bencher::Benchmark;
pub use runner::run;
