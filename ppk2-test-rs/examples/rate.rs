//!
//! An example that measures the power consumption of an example with different
//! measurement rates.
//!

use ppk2_test_rs::{Rate, Setup};
use std::{path::Path, process::Command, time::Duration};

const EXPERIMENT: &str = "exp_measure_micros";
const PATH: &str = "../experiments";
const DURATION: Duration = Duration::from_secs(2);

fn main() {
    let mut setup = Setup::new(None, Rate::FINE);

    setup.flash(
        Path::new(PATH),
        Command::new("cargo")
            .arg("flash")
            .arg("--chip")
            .arg("nRF52840_xxAA")
            .arg("--release")
            .arg("--bin")
            .arg(EXPERIMENT),
    );

    for i in 1..=10 {
        setup.rate = Rate::from_sps(i * 10_000);
        println!("RATE {}0_000\n{}", i, setup.measure(DURATION));
    }
}
