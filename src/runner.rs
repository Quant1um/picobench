use crate::{bench::Benchmark, stats};

pub fn run() {
    let mut result = vec![];

    if let Some(cores) = core_affinity::get_core_ids()
        && let Some(core) = cores.first()
    {
        core_affinity::set_for_current(*core);
    }

    for benchmark in Benchmark::list() {
        let (current, iters) = benchmark.run();
        let previous = cli::load_benchmark(&benchmark.name);
        cli::save_benchmark(&benchmark.name, &current);

        let mean = stats::Sample::from_iter(current.iter().copied()).mean();
        let (lower, upper) = stats::bootstrap_ci(&current, 10000, 0.05);

        let change = if let Some(previous) = &previous {
            let previous_mean = stats::Sample::from_iter(previous.iter().copied()).mean();
            let percentage = (mean - previous_mean) / previous_mean * 100.0;

            let p_value = stats::bootstrap_htest(&current, previous, 10000, false);
            if p_value < 0.005 {
                BenchmarkChange::Significant(percentage)
            } else {
                BenchmarkChange::Insignificant(percentage)
            }
        } else {
            BenchmarkChange::None
        };

        result.push(BenchmarkResult {
            path: benchmark.name.split("::").map(|s| s.to_string()).collect(),
            mean,
            lower,
            upper,
            iters,
            change,
        });
    }

    print::print(result);
}

#[derive(Clone, Debug)]
pub struct BenchmarkResult {
    pub path: Vec<String>,
    /// in milliseconds
    pub mean: f64,
    /// 95% confidence interval lower bound for the mean
    pub lower: f64,
    /// 95% confidence interval upper bound for the mean
    pub upper: f64,
    /// total number of iterations
    pub iters: u64,
    /// Change from previous benchmark, if any
    pub change: BenchmarkChange,
}

#[derive(Clone, Debug)]
pub enum BenchmarkChange {
    None,
    Insignificant(f64),
    Significant(f64),
}

mod cli {
    use std::fs::File;
    use std::io::{BufWriter, Write};
    use std::{
        collections::HashMap, env, path::PathBuf, process::Command, str::FromStr, sync::OnceLock,
    };
    use tinyjson::JsonValue;

    /// Returns the output directory for the benchmark results.
    pub fn output_directory() -> &'static PathBuf {
        static DIRECTORY: OnceLock<PathBuf> = OnceLock::new();
        DIRECTORY.get_or_init(|| {
            let path = if let Some(path) = env::var_os("PICOBENCH_HOME") {
                PathBuf::from(path)
            } else if let Some(path) = env::var_os("CARGO_TARGET_DIR") {
                PathBuf::from(path).join("picobench")
            } else if let Some(path) = cargo_metadata_target_directory() {
                path.join("picobench")
            } else {
                PathBuf::from("target/picobench")
            };

            let _ = std::fs::create_dir_all(&path);
            path
        })
    }

    /// Uses `cargo metadata` to get the target directory of the current project.
    pub fn cargo_metadata_target_directory() -> Option<PathBuf> {
        let mut cmd = Command::new("cargo");
        cmd.args(["metadata", "--format-version=1", "--no-deps"]);

        let output = cmd.output().ok()?;
        if !output.status.success() {
            return None;
        }

        let output = String::from_utf8_lossy(&output.stdout);
        let output = JsonValue::from_str(&output).ok()?;

        let target_directory = output
            .get::<HashMap<String, JsonValue>>()?
            .get("target_directory")?
            .get::<String>()?;

        Some(PathBuf::from(target_directory))
    }

    /// Saves the benchmark results to a file in the output directory.
    pub fn save_benchmark(name: &str, result: &[f64]) {
        let path = output_directory().join(format!("{}.txt", name.replace("::", "/")));
        std::fs::create_dir_all(path.parent().unwrap()).ok();

        let mut writer = BufWriter::new(File::create(path).unwrap());
        for result in result {
            writeln!(writer, "{:.9}", result).unwrap();
        }
    }

    /// Loads the benchmark results from a file in the output directory.
    pub fn load_benchmark(name: &str) -> Option<Vec<f64>> {
        let path = output_directory().join(format!("{}.txt", name.replace("::", "/")));
        let content = std::fs::read_to_string(path).ok()?;
        Some(
            content
                .lines()
                .filter_map(|line| line.trim().parse().ok())
                .collect(),
        )
    }
}

mod print {
    use crate::runner::{BenchmarkChange, BenchmarkResult};
    use std::{collections::BTreeMap, fmt::Display, ops::Range};

    /// A tree of benchmark results, used for grouping benchmarks by their module path.
    #[derive(Clone, Debug)]
    enum BenchmarkTree {
        Benchmark(BenchmarkResult),
        Group(String, Vec<BenchmarkTree>),
    }

    impl BenchmarkTree {
        fn from(results: Vec<BenchmarkResult>) -> Vec<BenchmarkTree> {
            let mut subgroups = BTreeMap::new();
            let mut leftover = vec![];

            for mut result in results {
                if result.path.len() <= 1 {
                    leftover.push(result);
                    continue;
                }

                let prefix = result.path.remove(0);
                subgroups
                    .entry(prefix)
                    .or_insert_with(Vec::new)
                    .push(result);
            }

            leftover.sort_unstable_by(|a, b| a.path.cmp(&b.path));

            let mut result = leftover
                .into_iter()
                .map(BenchmarkTree::Benchmark)
                .collect::<Vec<_>>();

            for (prefix, results) in subgroups {
                result.push(BenchmarkTree::Group(prefix, BenchmarkTree::from(results)));
            }

            result
        }
    }

    struct Cell {
        pub style: yansi::Style,
        pub right: bool,
        pub text: String,
    }

    impl Cell {
        pub fn new(text: impl Display) -> Self {
            Self {
                style: yansi::Style::new(),
                text: text.to_string(),
                right: true,
            }
        }

        pub fn left(mut self) -> Self {
            self.right = false;
            self
        }

        pub fn bold(mut self) -> Self {
            self.style = self.style.bold();
            self
        }

        pub fn dim(mut self) -> Self {
            self.style = self.style.dim();
            self
        }

        pub fn red(mut self) -> Self {
            self.style = self.style.red();
            self
        }

        pub fn green(mut self) -> Self {
            self.style = self.style.green();
            self
        }
    }

    fn time(ms: f64) -> String {
        if ms < 1e-6 {
            format!("{:.3}ps", ms * 1e9)
        } else if ms < 1e-3 {
            format!("{:.3}ns", ms * 1e6)
        } else if ms < 1e0 {
            format!("{:.3}µs", ms * 1e3)
        } else if ms < 1e3 {
            format!("{:.3}ms", ms)
        } else {
            format!("{:.3}s", ms)
        }
    }

    fn count(count: u64) -> String {
        if count < 10_000 {
            format!("{}", count)
        } else if count < 1_000_000 {
            format!("{:.1}k", count as f64 / 1e3)
        } else if count < 1_000_000_000 {
            format!("{:.1}m", count as f64 / 1e6)
        } else if count < 1_000_000_000_000 {
            format!("{:.1}b", count as f64 / 1e9)
        } else if count < 1_000_000_000_000_000 {
            format!("{:.1}t", count as f64 / 1e12)
        } else {
            "inf".to_string()
        }
    }

    fn print_tree_lines(depth: Range<usize>, is_last: bool, name: &str) -> String {
        let mut bars = String::new();
        for i in 0..depth.end {
            if i == depth.end - 1 {
                bars.push_str(if is_last { "╰─ " } else { "├─ " });
            } else {
                bars.push_str(if i >= depth.start { "│  " } else { "   " });
            }
        }
        bars.push_str(name);
        bars
    }

    fn print_tree(
        tree: BenchmarkTree,
        depth: Range<usize>,
        is_first: bool,
        is_last: bool,
    ) -> Vec<Vec<Cell>> {
        match tree {
            BenchmarkTree::Benchmark(result) => {
                let path = result.path.join("::");

                vec![vec![
                    Cell::new(print_tree_lines(depth, is_last, &path)).left(),
                    Cell::new(time(result.lower)),
                    Cell::new(time(result.mean)).bold(),
                    Cell::new(time(result.upper)),
                    match result.change {
                        BenchmarkChange::None => Cell::new(""),
                        BenchmarkChange::Insignificant(p) => Cell::new(format!("{:+.2}%", p)).dim(),
                        BenchmarkChange::Significant(p) if p > 0.0 => {
                            Cell::new(format!("{:+.2}%", p)).bold().red()
                        }
                        BenchmarkChange::Significant(p) => {
                            Cell::new(format!("{:+.2}%", p)).bold().green()
                        }
                    },
                    Cell::new(count(result.iters)),
                ]]
            }
            BenchmarkTree::Group(name, children) => {
                let mut rows = if is_first {
                    vec![vec![
                        Cell::new(print_tree_lines(depth.clone(), is_last, &name))
                            .bold()
                            .left(),
                        Cell::new("lower").dim(),
                        Cell::new("mean").dim(),
                        Cell::new("upper").dim(),
                        Cell::new("change").dim(),
                        Cell::new("iters").dim(),
                    ]]
                } else {
                    vec![vec![
                        Cell::new(print_tree_lines(depth.clone(), is_last, &name)).left(),
                    ]]
                };

                let last_child = children.len() - 1;
                for (i, child) in children.into_iter().enumerate() {
                    let last_child = i == last_child;
                    rows.extend(print_tree(
                        child,
                        (depth.start + if last_child && is_last { 1 } else { 0 })..(depth.end + 1),
                        false,
                        last_child,
                    ));
                }

                rows
            }
        }
    }

    pub fn print(results: Vec<BenchmarkResult>) {
        let mut table = vec![];

        let tree = BenchmarkTree::from(results.clone());
        for (i, tree) in tree.into_iter().enumerate() {
            table.extend(print_tree(tree, 0..0, i == 0, true));
        }

        let widths = (0..table.first().map_or(0, |f| f.len()))
            .map(|i| {
                table
                    .iter()
                    .filter_map(|row| row.get(i))
                    .map(|cell| cell.text.chars().count())
                    .max()
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();

        for row in table {
            for (cell, width) in row.into_iter().zip(widths.iter()) {
                let painted = yansi::Painted {
                    value: cell.text,
                    style: cell.style,
                };

                if cell.right {
                    print!("{:>width$}  ", painted, width = width);
                } else {
                    print!("{:<width$}  ", painted, width = width);
                }
            }
            println!();
        }
    }
}
