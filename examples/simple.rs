fn main() {
    picobench::run();
}

mod wait {
    mod busy {
        use std::time::Duration;

        fn spin(x: Duration) {
            let start = std::time::Instant::now();
            while start.elapsed() < x {}
        }

        #[picobench::bench]
        fn random_25_20_0() {
            if fastrand::bool() && fastrand::bool() {
                spin(Duration::from_millis(20));
            } else {
                spin(Duration::from_millis(0));
            }
        }

        #[picobench::bench]
        fn fixed_5() {
            spin(Duration::from_millis(5));
        }
    }

    mod spin {
        use std::time::Duration;

        fn spin(x: Duration) {
            let start = std::time::Instant::now();
            while start.elapsed() < x {
                std::hint::spin_loop();
            }
        }

        #[picobench::bench]
        fn random_25_20_0() {
            if fastrand::bool() && fastrand::bool() {
                spin(Duration::from_millis(20));
            } else {
                spin(Duration::from_millis(0));
            }
        }

        #[picobench::bench]
        fn fixed_5() {
            spin(Duration::from_millis(5));
        }
    }

    mod sleep {
        use std::thread::sleep;
        use std::time::Duration;

        #[picobench::bench]
        fn random_25_20_0() {
            if fastrand::bool() && fastrand::bool() {
                sleep(Duration::from_millis(20));
            } else {
                sleep(Duration::from_millis(0));
            }
        }

        #[picobench::bench]
        fn fixed_5() {
            sleep(Duration::from_millis(5));
        }
    }
}

mod mul {
    use std::{hint::black_box, time::Duration};

    #[picobench::bench]
    fn mul_1000() {
        let x = black_box((0..1000).map(|i| i as f64).collect::<Vec<_>>());
        let y = black_box((0..1000).map(|i| i as f64).collect::<Vec<_>>());

        for (a, b) in x.iter().zip(y.iter()) {
            black_box(a * b);
        }
    }

    #[picobench::bench(sample_time = Duration::from_millis(1000), sample_size = 10)]
    fn mul_1() {
        let i = black_box(0.0f64);
        let j = black_box(1.0f64);
        black_box(i * j);
    }
}

#[picobench::bench]
fn nothing() {}

#[picobench::bench]
fn almost_nothing() {
    std::hint::black_box(());
}
