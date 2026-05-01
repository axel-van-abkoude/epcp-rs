#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

pub mod types;
use crate::types::*;

// We use the ppk2-rs library to interface with the Ppk2
use ppk2::{
    Ppk2,
    measurement::MeasurementMatch,
    try_find_ppk2_port,
    types::{DevicePower, MeasurementMode},
};

// Used for time management
use std::{
    env::set_current_dir,
    path::Path,
    process::Command,
    sync::mpsc::RecvTimeoutError,
    time::{Duration, Instant},
};

/// The experiment setup.
/// Create a new setup with [Setup::new]
/// Flash a device with a custom flash script with [Setup::flash]
/// Then measure with [Setup::measure] which returns a [Sections] object.
pub struct Setup {
    /// The ppk2 is wrapped in an Option type to keep it live during the lifetime
    /// of Setup. When Ppk2 is moved (in [Ppk2::start_measurement]) we take the
    /// value from [Setup::ppk2] leaving a None value. When the measurement is
    /// completed we put it back. This is done with the appropriatly named
    /// [Setup::take] and [Setup::put] functions.
    ppk2: Option<Ppk2>,
    /// The rate that will be measured with
    /// Will not update the rate of a measurement while mid measurement
    pub rate: Rate,
}

/// All functionality in one test to keep the lifetime of the ppk2 alive
/// Needed to make the ppk2 not shut off when borrowed
impl Setup {
    const DETECT_POWER_THRESHOLD: f32 = 500.0;

    /// Creates a new setup which tries to connect to a ppk2 device if a port is
    /// provided. If not it tries to find a connected ppk2 device.
    pub fn new(ppk2_port: Option<String>, rate: Rate) -> Setup {
        let serial_port = match ppk2_port {
            Some(p) => p,
            None => try_find_ppk2_port().unwrap(),
        };

        let mut ppk2 = Ppk2::new(serial_port.clone(), MeasurementMode::Ampere).unwrap();
        //ppk2.set_source_voltage(SourceVoltage::from_millivolts(800)).unwrap();
        ppk2.set_device_power(DevicePower::Disabled).unwrap();

        Setup {
            ppk2: Some(ppk2),
            rate: rate,
        }
    }

    /// Flashes the device with a given flash_script
    /// This flash script is executed in a tmp folder thus when flashing from
    /// source a 'git clone' or a 'cd' to the directory of the embedded project
    /// is needed (see examples).
    pub fn flash(&mut self, path_to_project_dir: &Path, flash_command: &mut Command) {
        // We enable the power to the device and wait untill it detects power
        self.power_enable_detect();

        set_current_dir(path_to_project_dir).unwrap();
        let status = flash_command.status().expect("COMMAND FAILED");

        self.power_disable();
        if !status.success() {
            panic!("{}", status);
        } else {
            println!("{}", status);
        }
    }

    /// Starts a measurement for a certain duration
    pub fn measure(&mut self, duration: Duration) -> Sections {
        self.power_enable_detect();
        let ppk2 = self.take();
        let (rcv, stop) = ppk2.start_measurement(self.rate.as_usize()).unwrap();

        let mut sections = Sections::new();
        let init = Instant::now();
        let mut end = init;

        loop {
            // Check if the end of the previous average has exceeded the time limit
            // Enforces no measurement when duration is 0.0
            if end.duration_since(init) > duration {
                break;
            }

            // Get the previous ending timestamp and label it as the start of this
            // measurement.
            let start = end;

            // Blocking call recv_timeout
            let rcv_res = rcv.recv_timeout(Duration::from_secs(2));

            // Get the timestamp of receiving the measurements and label it as
            // the end of the measurement
            end = Instant::now();

            // Handle the received response
            match rcv_res {
                Ok(MeasurementMatch::Match(m)) => {
                    sections.update_with(m, end.duration_since(start))
                }
                Ok(MeasurementMatch::NoMatch) => {
                    todo!("we match on everything always thus this should never happen")
                }
                Err(RecvTimeoutError::Disconnected) => break,
                Err(_) => todo!("handle"),
            }
        }
        self.stop_and_put(stop);
        sections
    }

    /// Waits until power has been detected and measured
    pub fn power_enable_detect(&mut self) {
        self.power_enable();
        let ppk2 = self.take();
        let (rcv, stop) = ppk2.start_measurement(self.rate.as_usize()).unwrap();
        loop {
            match rcv.recv_timeout(Duration::from_secs(2)) {
                Ok(MeasurementMatch::Match(m)) if m.micro_amps > Self::DETECT_POWER_THRESHOLD => {
                    break;
                }
                Ok(_) => continue,
                Err(_) => todo!("Error in wait_until_power"),
            }
        }
        self.stop_and_put(stop)
    }

    /// Enables the power on the ppk2 device
    pub fn power_enable(&mut self) {
        let mut ppk2 = self.take();
        ppk2.set_device_power(DevicePower::Enabled).unwrap();
        self.put(ppk2);
    }

    /// Disables the power on the ppk2 device
    pub fn power_disable(&mut self) {
        let mut ppk2 = self.take();
        ppk2.set_device_power(DevicePower::Disabled).unwrap();
        self.put(ppk2);
    }

    // Borrow the ppk2 and leave a None value such that it can not be accessed
    // by two functions at once
    fn take(&mut self) -> Ppk2 {
        self.ppk2.take().unwrap()
    }

    // Release the ppk2 and put it back for other functions to use
    fn put(&mut self, ppk2: Ppk2) {
        self.ppk2 = Some(ppk2);
    }

    /// As Ppk2 is moved in [Ppk2::start_measurement] we need to handle it
    /// keep it alive. This is done via Option::take()
    fn stop_and_put(&mut self, stop: impl FnOnce() -> Result<Ppk2, ppk2::Error>) {
        self.put(stop().unwrap());
    }
}

/// The rate of samples of the ppk2 in samples per second
/// Ranges between [Rate::MIN_SPS] and [Rate::MAX_SPS].
#[derive(Copy, Clone)]
pub struct Rate(u32);

impl Rate {
    /// Constant value which represents the *minimum* samples per second that can
    /// be passed to the ppk2.
    pub const MIN_SPS: u32 = 1;

    /// Constant value which represents the *maximum* samples per second that can
    /// be passed to the ppk2.
    pub const MAX_SPS: u32 = 100_000;

    /// Rate which results in a fine granularity in measurements
    ///
    /// (+) More accurate
    /// (+) Can spot outliers with effects on powerconsumption > 10 µseconds
    /// (-) Higher storage usage
    /// (-) Outliers can skew metrics like averages
    pub const FINE: Rate = Rate(Rate::MAX_SPS);

    /// Rate which results in a coarse granularity in measurements
    ///
    /// (-) Less accurate
    /// (-) It is harder to spot single instruction outliers
    /// (+) Lower storage usage
    /// (+) Good for comparing average loads
    pub const COARSE: Rate = Rate(10_000);

    /// Rate data constructor
    /// Rejects values that lie outside of the range
    /// [Rate::MIN_SPS] ..= [Rate::MAX_SPS]
    pub fn from_sps(sps: u32) -> Rate {
        match sps {
            Rate::MIN_SPS..=Rate::MAX_SPS => Rate(sps),
            x => todo!("sample size out of bounds: {}", x),
        }
    }

    /// Returns the rate as samples per second in u32
    pub fn as_u32(self) -> u32 {
        self.0
    }

    /// Returns the rate as samples per second in usize
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}
