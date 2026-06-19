//! Statistics functions for analyzing benchmark results.

/// Computes the 50th, 95th and 99th percentiles of a sample.
pub fn percentile_50_95_99(data: impl IntoIterator<Item = f64>) -> (f64, f64, f64) {
    let mut sorted: Vec<_> = data.into_iter().collect();
    sorted.sort_unstable_by(|a, b| a.total_cmp(b));
    let p50 = sorted[(0.50 * sorted.len() as f64) as usize];
    let p95 = sorted[(0.95 * sorted.len() as f64) as usize];
    let p99 = sorted[(0.99 * sorted.len() as f64) as usize];
    (p50, p95, p99)
}

/// Performs a two-sample Mood's median test to compare the medians of two samples.
///
/// Returns the chi-squared statistic for the null hypothesis that the two samples have the same population median.
pub fn median_test(data1: &[f64], data2: &[f64]) -> f64 {
    let n1 = data1.len();
    let n2 = data2.len();

    let (median, _, _) = percentile_50_95_99(data1.iter().chain(data2.iter()).copied());
    let above1 = data1.iter().filter(|&&x| x > median).count();
    let above2 = data2.iter().filter(|&&x| x > median).count();
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

#[cfg(test)]
mod tests {

    #[test]
    fn median_test() {
        let data1 = [
            1.0, 14.0, 19.0, 12.0, 11.0, 15.0, 20.0, 5.0, 21.0, 15.0, 15.0, 28.0, 3.0, 6.0,
        ];
        let data2 = [
            16.0, 17.0, 19.0, 10.0, 31.0, 22.0, 26.0, 24.0, 27.0, 32.0, 14.0, 8.0, 12.0, 11.0,
        ];

        let chi2 = super::median_test(&data1, &data2);
        assert!((chi2 - 3.59).abs() < 0.01);
    }

    #[test]
    fn percentile() {
        let data = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
            17.0, 18.0, 19.0, 20.0, 21.0,
        ];
        let (p50, p95, p99) = super::percentile_50_95_99(data);
        assert_eq!(p50, 11.0);
        assert_eq!(p95, 20.0);
        assert_eq!(p99, 21.0);
    }
}
