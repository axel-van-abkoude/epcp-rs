//!
//! Only works for nRF devkits, only tested on nRF52840-DK
//!
//! USE AT OWN RISK!
//!
//! uses nrfutil
//! https://docs.nordicsemi.com/bundle/nrfutil/page/guides/installing.html
//!

use ppk2_test_rs::{Rate, Setup};
use std::{io::stdin, path::Path, process::Command};

/// Traits to filter devices on
/// Traits per device can be found by running:
/// ```
/// nrfutil device list
/// ```
const TRAITS: &str = "devkit";

fn main() {
    let mut setup = Setup::new(None, Rate::FINE);

    // First list the devices with the devkit trait
    setup.flash(
        Path::new("."),
        Command::new("nrfutil")
            .arg("device")
            .arg("list") // <- list
            .arg("--traits")
            .arg(TRAITS),
    );

    println!("Recover these devices (y/n)?");

    // Interrupt driven (No busy wait)
    let mut buffer = String::new();
    stdin().read_line(&mut buffer).unwrap();

    // Catches 'y' or 'n'. Otherwise loops.
    match buffer.chars().nth(0).unwrap() {
        'y' => {
            setup.flash(
                Path::new("."),
                Command::new("nrfutil")
                    .arg("device")
                    .arg("recover") // <- recover
                    .arg("--traits")
                    .arg(TRAITS),
            );
            println!("Recovered. Bye!")
        }
        _ => println!("Not recovering. Bye!"),
    }
}
