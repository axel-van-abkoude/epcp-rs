//!
//! An example that measures the power consumption of an example with different
//! measurement rates.
//!

use ppk2_test_rs::{Setup, When::*, types::Pins};
use std::{path::Path, process::Command, time::Duration};

// Create a boxed value
macro_rules! bx {
    ($e:expr) => {
        Box::new($e)
    };
}

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

    setup.wait_until(Time(WARMUP));
    setup.wait_until(Logic(all_zero));

    // Measure from a non 0 pin configuration until 0 has been found
    println!(
        "{}",
        setup.measure(Not(bx!(Logic(all_zero))), Logic(all_zero))
    );
}
