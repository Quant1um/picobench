/// Computes the 95th and 99th percentiles of a sample.
pub fn percentile_95_99(data: &[f64]) -> (f64, f64) {
    let mut sorted = data.to_vec();
    sorted.sort_unstable_by(|a, b| a.total_cmp(b));
    let p95 = sorted[(0.95 * sorted.len() as f64) as usize];
    let p99 = sorted[(0.99 * sorted.len() as f64) as usize];
    (p95, p99)
}

/// Computes the p-value for comparing two samples using bootstrapping.
/// Returns the p-value for the null hypothesis that the first sample has the same mean as the second sample.
pub fn bootstrap_htest(data1: &[f64], data2: &[f64], n_resamples: usize, one_tailed: bool) -> f64 {
    #[inline(always)]
    fn t_statistic(mean1: f64, mean2: f64, var1: f64, var2: f64, inv_n1: f64, inv_n2: f64) -> f64 {
        (mean1 - mean2) / (var1 * inv_n1 + var2 * inv_n2).sqrt()
    }

    let mut rng = fastrand::Rng::new();

    let sample1 = Sample::from(data1);
    let sample2 = Sample::from(data2);

    let offset1 = sample1.join(sample2).mean() - sample1.mean();
    let offset2 = sample1.join(sample2).mean() - sample2.mean();

    let inv_n1 = 1.0 / sample1.n as f64;
    let inv_n2 = 1.0 / sample2.n as f64;

    let t = t_statistic(
        sample1.mean(),
        sample2.mean(),
        sample1.variance(),
        sample2.variance(),
        inv_n1,
        inv_n2,
    );

    let mut passed = 0;
    for _ in 0..n_resamples {
        let sample1 = Sample::resample(&mut rng, data1);
        let sample2 = Sample::resample(&mut rng, data2);

        let sample_t = t_statistic(
            sample1.mean() + offset1,
            sample2.mean() + offset2,
            sample1.variance(),
            sample2.variance(),
            inv_n1,
            inv_n2,
        );

        if sample_t >= t {
            passed += 1;
        }
    }

    let p_value = passed as f64 / n_resamples as f64;

    if one_tailed {
        p_value
    } else {
        p_value.min(1.0 - p_value) * 2.0
    }
}

/// A sample of values, assumed to be gaussian,
/// used for calculating statistics such as mean, standard deviation, and confidence intervals.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub n: u64,
    pub sum: f64,
    pub sum2: f64,
}

impl Sample {
    /// Single bootstrap resample with replacement of a sample.
    #[inline(always)]
    pub fn resample(rng: &mut fastrand::Rng, sample: &[f64]) -> Self {
        let mut sum = 0.0;
        let mut sum2 = 0.0;

        for _ in 0..sample.len() {
            let x = sample[rng.usize(..sample.len())];
            sum += x;
            sum2 += x * x;
        }

        Self {
            n: sample.len() as u64,
            sum,
            sum2,
        }
    }

    /// Joins this sample with another sample, returning a new sample that represents the combined data.
    pub fn join(&self, other: Self) -> Self {
        Self {
            n: self.n + other.n,
            sum: self.sum + other.sum,
            sum2: self.sum2 + other.sum2,
        }
    }

    /// Returns the mean of the sample.
    pub fn mean(&self) -> f64 {
        self.sum / self.n as f64
    }

    /// Returns the variance of the sample (unbiased estimate).
    pub fn variance(&self) -> f64 {
        if self.n <= 1 {
            return 0.0;
        }

        (self.sum2 - self.sum.powi(2) / self.n as f64) / (self.n as f64 - 1.0)
    }

    /// Returns the 95% confidence interval half-size for the mean of the sample.
    pub fn stderr95(&self) -> f64 {
        1.96 * (self.variance() / self.n as f64).sqrt()
    }
}

impl Default for Sample {
    fn default() -> Self {
        Self {
            n: 0,
            sum: 0.0,
            sum2: 0.0,
        }
    }
}

impl FromIterator<f64> for Sample {
    fn from_iter<I: IntoIterator<Item = f64>>(iter: I) -> Self {
        let mut n = 0u64;
        let mut sum = 0.0;
        let mut sum2 = 0.0;

        for x in iter {
            n += 1;
            sum += x;
            sum2 += x * x;
        }

        Self { n, sum, sum2 }
    }
}

impl From<&[f64]> for Sample {
    fn from(values: &[f64]) -> Self {
        values.iter().copied().collect()
    }
}

impl From<f64> for Sample {
    fn from(value: f64) -> Self {
        Self {
            n: 1,
            sum: value,
            sum2: value * value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_htest() {
        let sample1 = vec![-1000.0, -400.0, -1000.0];
        let sample2 = vec![
            -1.0, -2.0, -3.0, -4.0, -5.0, 100.0, 200.0, 300.0, 400.0, 500.0,
        ];
        let p_value = bootstrap_htest(&sample1, &sample2, 10000, false);
        println!("Bootstrap H-test p-value: {}", p_value);
    }
}
