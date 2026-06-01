//!
//! An example that measures the power consumption of an example with different
//! measurement rates.
//!

use ppk2_test_rs::{
    Rate, Setup,
    logic::{Pins, When::*},
};
use std::{path::Path, process::Command, time::Duration};

const EXPERIMENT: &str = "pin_influence";
const PATH: &str = "../experiments";
const WARMUP: Duration = Duration::from_secs(1);

fn main() {
    let mut setup = Setup::find();

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

    let all_zero = Pins::from(0u8);

    setup.wait_until(Time(WARMUP)& Logic(all_zero));

    // Run with sample sizes 10_000 to 100_000 with intervals of 1_000
    for i in 1..=10 {
        setup.rate = Rate::from_sps(i * 10_000);
        println!(
            "{}^^^^^^ RATE {}0_000 ^^^^^^\n",
            setup.measure(!Logic(all_zero), Logic(all_zero)),
            i
        );
    }
}
