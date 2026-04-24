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

use std::{
    sync::mpsc::RecvTimeoutError,
    time::{Duration, Instant},
};

// Used for flashing
use run_script::run_script;

/// The experiment setup.
/// Create a new setup with [Setup::new]
/// Flash a device with a custom flash script with [Setup::flash]
/// Then measure with [Setup::measure] which returns a [Sections] object.
pub struct Setup {
    ppk2: Option<Ppk2>,
    rate: Rate,
}

/// All functionality in one test to keep the lifetime of the ppk2 alive
/// Needed to make the ppk2 not shut off when borrowed
impl Setup {
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
    pub fn flash(&mut self, flash_script: &str) {
        self.power_enable();
        self.wait_until_power(500.0);
        // Defaults to sh on unix and cmd on windows
        let (code, output, error) = run_script!(flash_script).unwrap();

        println!("EXIT:\n{}", code);
        println!("OUTPUT:\n{}", output);
        println!("ERROR:\n{}", error);

        if code != 0 {
            todo!("Handle a non-0 exit status for flashing");
        }
        self.power_disable();
    }

    /// Starts a measurement for a certain duration
    pub fn measure(&mut self, duration: Duration) -> Result<Sections, RecvTimeoutError> {
        let ppk2 = self.take();
        let (rcv, stop) = ppk2.start_measurement(self.rate.as_usize()).unwrap();

        let mut sections = Sections::new();
        let start = Instant::now();
        let mut prev = start.clone();

        let ret = loop {
            // Check if the end of the previous average has exceeded the time limit
            // Enforces no measurement when duration is 0.0
            if prev.duration_since(start) > duration {
                break Ok(sections);
            }

            // Blocking call recv_timeout
            let rcv_res = rcv.recv_timeout(Duration::from_secs(2));

            // Only get time after the measurement has come in
            // now - prev is the time measured
            let now = Instant::now();

            match rcv_res {
                Ok(MeasurementMatch::Match(m)) => {
                    let mut section = sections[Pins::from(m.pins)];
                    let duration_span = now.duration_since(prev);

                    // µA * µs = A*s*(µ^2)
                    // h:  sec / 60^2
                    section.total_capacity += m.micro_amps * (duration_span.as_micros() as f32);
                    section.total_duration += duration_span;

                    prev = now;
                    sections[Pins::from(m.pins)] = section;
                }
                Ok(MeasurementMatch::NoMatch) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    break Ok(sections);
                }
                Err(e) => {
                    break Err(e);
                }
            }
        };
        self.stop_and_put(stop);
        ret
    }

    /// Waits until a certain µA have been measured
    pub fn wait_until_power(&mut self, micro_amps: f32) {
        let ppk2 = self.take();
        let (rcv, stop) = ppk2.start_measurement(self.rate.as_usize()).unwrap();
        let ret = loop {
            match rcv.recv_timeout(Duration::from_secs(2)) {
                Ok(MeasurementMatch::Match(m)) if m.micro_amps > micro_amps => break,
                // Ok(MeasurementMatch::Match(m)) => {
                //     println!("{}",m.micro_amps);
                //     continue
                // },
                Ok(_) => continue,
                Err(_) => {
                    self.stop_and_put(stop);
                    todo!("Error in wait_until_power")
                }
            }
        };
        self.stop_and_put(stop);
        ret
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
    /// Retrieves the rate that is set in Setup
    pub fn get_rate(&mut self) -> Rate {
        self.rate
    }

    /// Sets the rate, does not update rate mid measurement
    pub fn set_rate(&mut self, rate: Rate) {
        self.rate = rate;
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
        self.put(match stop() {
            Ok(ppk2) => ppk2,
            Err(_) => todo!("handle error in stop_and_put"),
        });
    }
}

/// The rate of samples of the ppk2 in samples per second
/// Ranges between [Rate::MIN_SPS] and [Rate::MAX_SPS].
#[derive(Copy, Clone)]
pub struct Rate(u32);

impl Rate {
    /// Constant value which represents the *minimum* samples per second that can
    /// be passed to the ppk2.
    const MIN_SPS: u32 = 1;

    /// Constant value which represents the *maximum* samples per second that can
    /// be passed to the ppk2.
    const MAX_SPS: u32 = 100_000;

    /// Rate which results in a fine granularity in measurements
    /// ```
    /// + More accurate
    /// + Can spot outliers with effects on powerconsumption > 10 µseconds
    /// - Higher storage usage
    /// - Outliers can skew metrics like averages
    /// ```
    pub const FINE: Rate = Rate(Rate::MAX_SPS);

    /// Rate which results in a coarse granularity in measurements
    /// ```
    /// - Less accurate
    /// - It is harder to spot single instruction outliers
    /// + Lower storage usage
    /// + Good for comparing average loads
    /// ```
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
