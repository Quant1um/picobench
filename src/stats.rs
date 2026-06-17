/// Computes the 50th, 95th and 99th percentiles of a sample.
pub fn percentile_50_95_99(data: &[f64]) -> (f64, f64, f64) {
    let mut sorted = data.to_vec();
    sorted.sort_unstable_by(|a, b| a.total_cmp(b));
    let p50 = sorted[(0.50 * sorted.len() as f64) as usize];
    let p95 = sorted[(0.95 * sorted.len() as f64) as usize];
    let p99 = sorted[(0.99 * sorted.len() as f64) as usize];
    (p50, p95, p99)
}

/// Computes the chi-squared statistic for the median test on two samples.
///
/// Returns the chi-squared statistic for the null hypothesis that the two samples have the same population median.
pub fn median_test(data1: &[f64], data2: &[f64]) -> f64 {
    let n1 = data1.len();
    let n2 = data2.len();

    let mut combined = Vec::with_capacity(n1 + n2);
    combined.extend(data1.iter().copied());
    combined.extend(data2.iter().copied());
    combined.sort_unstable_by(|a, b| a.total_cmp(b));

    let median = combined[combined.len() / 2];

    let above1 = data1.iter().filter(|&&x| x < median).count();
    let above2 = data2.iter().filter(|&&x| x < median).count();
    let below1 = n1 - above1;
    let below2 = n2 - above2;

    let eb1 = (below1 as f64 + below2 as f64) * (n1 as f64) / ((n1 + n2) as f64);
    let eb2 = (below1 as f64 + below2 as f64) * (n2 as f64) / ((n1 + n2) as f64);
    let ea1 = (above1 as f64 + above2 as f64) * (n1 as f64) / ((n1 + n2) as f64);
    let ea2 = (above1 as f64 + above2 as f64) * (n2 as f64) / ((n1 + n2) as f64);

    ((below1 as f64 - eb1).powi(2) / eb1)
        + ((below2 as f64 - eb2).powi(2) / eb2)
        + ((above1 as f64 - ea1).powi(2) / ea1)
        + ((above2 as f64 - ea2).powi(2) / ea2)
}
