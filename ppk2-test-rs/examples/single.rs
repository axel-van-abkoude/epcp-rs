//!
//! An example that measures the power consumption of an example with different
//! measurement rates.
//!

use ppk2_test_rs::{
    Setup,
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

    let all_low = Pins::from(0u8);
    setup.wait_until(Time(WARMUP) & Logic(all_low));

    // Measure from a non 0 pin configuration until 0 has been found
    println!(
        "{}",
        setup.measure(!Logic(all_low), Logic(all_low))
    );
}
