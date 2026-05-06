//!
//! An example that flashes the device with an experiment.
//!

use ppk2_test_rs::{Rate, Setup};
use std::{path::Path, process::Command};

const EXPERIMENT: &str = "exp_measure_micros";
const PATH: &str = "../experiments";

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
}
