use ppk2_test_rs::{Rate, Setup};
use std::{path::Path, process::Command};

fn main() {
    let mut setup = Setup::new(None, Rate::FINE);
    setup.flash(
        Path::new("../experiments"),
        Command::new("cargo")
            .arg("flash")
            .arg("--chip")
            .arg("nRF52840_xxAA")
            .arg("--release")
            .arg("--bin")
            .arg("exp_measure_micros"),
    );
}
