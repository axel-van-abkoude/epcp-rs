use ppk2_test_rs::{Rate, Setup};
use std::{path::Path, process::Command, time::Duration};

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


    let duration = Duration::from_secs(2);
    for i in 1..=10 {
        setup.rate = Rate::from_sps(i * 10_000);
        println!("RATE {}0_000\n{}", i, setup.measure(duration));

    }
}
