// use ppk2_test_rs::Setup;

fn main() {

    // let mut setup = Setup::new(None, ppk2::types::MeasurementMode::Ampere, 100_000);
    // setup.power_enable();
    // setup = setup.flash();
    // setup.power_disable();
}
//
// #[cfg(test)]
// mod ppk2_measurement {
//     use crate::Args;
//     use anyhow::Result;
//     use clap::Parser;
//     use ppk2::{
//         Ppk2,
//         measurement::MeasurementMatch::*,
//         try_find_ppk2_port,
//         types::{DevicePower, LogicPortPins, Section, Sections},
//     };
//     use serial_test::serial;
//     use std::{
//         sync::mpsc::RecvTimeoutError,
//         thread,
//         time::{Duration, Instant},
//     };
//     use tracing::{debug, error, info};
//
//     #[test]
//     #[serial]
//     pub fn test_100_000() {
//         let args =
//             &Args::try_parse_from(["", "-e", "enabled", "-m", "ampere", "-s", "100000"]).unwrap();
//         let sections: Sections = ppk2_test(once, args);
//         print_sections(sections);
//         assert!(
//             sections[255].total_time > 0.0,
//             "We have measured something in section 255"
//         );
//     }
//
//     #[test]
//     #[serial]
//     pub fn test_x10_000() {
//         let args =
//             &Args::try_parse_from(["", "-e", "enabled", "-m", "ampere", "-s", "10000"]).unwrap();
//         let sections: Sections = ppk2_test(args);
//         print_sections(sections);
//         assert!(
//             sections[255].total_power > 0.0,
//             "We have measured something in section 255"
//         );
//     }
//
//     #[test]
//     #[serial]
//     pub fn test_xx1_000() {
//         let args =
//             &Args::try_parse_from(["", "-e", "enabled", "-m", "ampere", "-s", "1000"]).unwrap();
//         let sections: Sections = ppk2_test(args);
//         print_sections(sections);
//         assert!(
//             sections[255].total_power > 0.0,
//             "We have measured something in section 255"
//         );
//     }
//
//     #[test]
//     #[serial]
//     pub fn test_xxx_100() {
//         let args =
//             &Args::try_parse_from(["", "-e", "enabled", "-m", "ampere", "-s", "100"]).unwrap();
//         let sections: Sections = ppk2_test(args);
//         print_sections(sections);
//         assert!(
//             sections[255].total_power > 0.0,
//             "We have measured something in section 255"
//         );
//     }
// }
