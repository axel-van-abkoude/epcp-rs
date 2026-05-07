//!
//! An example that measures the power consumption of an example with different
//! measurement rates.
//!

use ppk2_test_rs::{Predicate, Rate, Setup, types::Pins};
use std::{path::Path, process::Command};

const EXPERIMENT: &str = "pin_influence";
const PATH: &str = "../experiments";

fn main() {
    let mut setup = Setup::new(None, Rate::FINE);

    // Flash device
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
    setup.power_enable();

    // Run with sample sizes 10_000 to 100_000 with intervals of 1_000
    for i in 1..=10 {
        let all_zero = Predicate::Pins(Pins::from(0u8));
        let not_all_zero = Predicate::Not(Box::new(all_zero.clone()));

        setup.rate = Rate::from_sps(i * 10_000);

        let _ = setup.measure(not_all_zero);
        println!("RATE {}0_000\n{}", i, setup.measure(all_zero));
    }
}
