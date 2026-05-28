#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

pub mod types;
use crate::types::*;

use csv::Writer;
// We use the ppk2-rs library to interface with the Ppk2
use ppk2::{
    Ppk2,
    measurement::MeasurementMatch,
    try_find_ppk2_port,
    types::{DevicePower, MeasurementMode},
};

// Used for time management
use std::{
    env::{current_dir, set_current_dir},
    fmt::Debug,
    io::{self, Write},
    path::Path,
    process::{Command, Stdio},
    sync::mpsc::RecvTimeoutError,
    time::{Duration, Instant},
};

// Local time for getting the current time as the std lib does not give a
// general way to get this.
use chrono::Local;

/// Macro to help with [Setup::flash]
/// Gets a stream of child process and displays it in the parent stdout
/// in a given format
macro_rules! pipe_fmt {
    ($stream:expr, $format:expr) => {
        if let Some(stream) = $stream.take() {
            let reader = std::io::BufReader::new(stream);
            for line in std::io::BufRead::lines(reader).flatten() {
                println!($format, line);
            }
        }
    };
}

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

    data_dir: String,
}

/// All functionality in one test to keep the lifetime of the ppk2 alive
/// Needed to make the ppk2 not shut off when borrowed
impl Setup {
    const TIMEOUT_DURATION: Duration = Duration::from_secs(2);

    /// Creates a new setup from a specified port with a [Rate::FINE] rate.

    pub fn new(ppk2_port: String) -> Setup {
        let mut ppk2 = Ppk2::new(ppk2_port, MeasurementMode::Ampere).unwrap();

        ppk2.set_device_power(DevicePower::Disabled).unwrap();

        Self::print_header();

        Setup {
            ppk2: Some(ppk2),
            rate: Rate::FINE,
            data_dir: String::from("./data"),
        }
    }

    /// Tries to find a ppk2_port and creates a new setup from it.
    pub fn find() -> Setup {
        Self::new(try_find_ppk2_port().unwrap())
    }

    /// Flashes the device with a given flash command from a specified path.
    ///
    /// Flashing while the ppk2 is connected without providing power will
    /// soft brick the target. As providing power is not instant this function
    /// will wait until it detects power.
    ///
    /// For the nRF52840 measuring a current greater than 0 is enough to detect
    /// if enough power is provided to flash. When the target device is soft
    /// bricked due to not having power one can look to use wait_until on a
    /// greater current.
    pub fn flash(&mut self, target_dir: &Path, flash_command: &mut Command) {
        self.power_enable();

        // We wait until we actually measure some power to continue.
        //
        // In the case of the nRF52840 we measure negative current
        // when the power is not provided to the board.
        self.wait_until(When::CurrentGt(Current::from_micros(0.0)));

        // We flash the device from the target directory and pipe stdout and
        // stderr of the child to capture it in the terminal.
        let original_dir = current_dir().unwrap();
        set_current_dir(target_dir).unwrap();
        let mut child = flash_command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawning flash_command");
        set_current_dir(original_dir).unwrap();

        println!("\n==============COMMAND OUTPUT================");

        pipe_fmt!(child.stderr, "[stderr] {}");
        pipe_fmt!(child.stdout, "[stdout] {}");

        // Wait for the child process to finish and give feedback on the code
        let exit_child = child.wait().unwrap();

        match exit_child.code() {
            Some(code) => println!("==============EXIT CODE {:<3}=================\n", code),
            None => println!("==============TERMINATED====================\n"),
        }

        self.power_disable();
    }

    /// Starts a measurement when a starting condition is true
    /// Stops when a stopping condition is true
    pub fn measure(&mut self, start: When, stop: When) -> Sections {
        let ppk2 = self.take();

        use MeasureStatus::*;
        let mut status = &Waiting;

        // Create a output csv file
        let data_path: String = format!(
            "{}/{}.csv",
            self.data_dir,
            Local::now().format("%Y-%m-%d-%H:%M:%S")
        );

        let mut csv_writer = Writer::from_path(data_path.clone()).unwrap();

        let mut sections = Sections::new();

        // Initialise timing where begin signifies the begin of Waiting and is
        // later updated with the beginning of Measuring
        let mut begin = Instant::now();
        // End sample needs to be remembered in the next iteration of the loop
        // thus it is declared here.
        let mut end_sample = begin;

        let (rcv, stop_ppk2) = ppk2.start_measurement(self.rate.as_usize()).unwrap();

        loop {
            // Mark the start and end of this received sample
            let begin_sample = end_sample;
            let rcv_res = rcv.recv_timeout(Self::TIMEOUT_DURATION);
            end_sample = Instant::now();

            // Capture the durations
            let duration_sample = end_sample.duration_since(begin_sample);
            let timestamp_sample = end_sample.duration_since(begin);

            // When data is found update the sections according to the predicates
            use MeasurementMatch::*;
            use RecvTimeoutError::*;
            match rcv_res {
                Ok(Match(m)) => {
                    let sample = &Sample {
                        timestamp: timestamp_sample,
                        duration: duration_sample,
                        current: Current::from_micros(m.micro_amps),
                        pins: Pins::from(m.pins),
                    };

                    Self::print_status(status, sample);

                    match status {
                        Waiting => {
                            // Update the status when the starting predicate is true
                            // Include the sample in the section as the predicate
                            // holds in the current sample
                            if start.eval(sample) {
                                println!(" => Starting Condition Found!");
                                status = &Measuring;
                                // update begin to the begin of the measurement
                                begin = end_sample;
                                sections.update_with(m, duration_sample);
                            }
                        }
                        Measuring => {
                            // Stop measuring if the stopping predicate is true

                            // Exclude the sample in the section as the predicate
                            // does not hold in the current sample
                            if stop.eval(sample) {
                                println!(" => Measurement Completed!");
                                break;
                            }

                            // Write to a csv file
                            csv_writer.serialize(sample).unwrap();

                            sections.update_with(m, duration_sample);
                        }
                    }
                }
                Ok(NoMatch) => {
                    todo!("we match on everything always thus this should never happen")
                }
                Err(Disconnected) => break,
                Err(_) => todo!("Error in measure"),
            }
        }
        println!("\nData written to: {}\n", data_path);
        self.stop_and_put(stop_ppk2);
        sections
    }

    /// Waits and stops when a stopping condition holds
    pub fn wait_until(&mut self, stop: When) {
        let ppk2 = self.take();
        let (rcv, stop_ppk2) = ppk2.start_measurement(Rate::COARSE.as_usize()).unwrap();
        let begin = Instant::now();
        let mut end_sample = begin;
        loop {
            let begin_sample = end_sample;
            let rcv_res = rcv.recv_timeout(Self::TIMEOUT_DURATION);
            end_sample = Instant::now();

            let duration_sample = end_sample.duration_since(begin_sample);
            let timestamp_sample = end_sample.duration_since(begin);

            use MeasureStatus::*;
            use MeasurementMatch::*;
            match rcv_res {
                // Stop if the stopping predicate holds
                Ok(Match(m)) => {
                    let sample = &Sample {
                        timestamp: timestamp_sample,
                        duration: duration_sample,
                        current: Current::from_micros(m.micro_amps),
                        pins: Pins::from(m.pins),
                    };
                    Self::print_status(&Waiting, sample);
                    if stop.eval(sample) {
                        break;
                    }
                }
                Ok(_) => continue,
                Err(_) => todo!("Error in wait_until"),
            }
        }
        println!(" => Stopped Waiting.");
        self.stop_and_put(stop_ppk2)
    }

    /// Enables the power on the ppk2 device
    /// This does not have an immediate effect on the target board
    pub fn power_enable(&mut self) {
        let mut ppk2 = self.take();
        ppk2.set_device_power(DevicePower::Enabled).unwrap();
        self.put(ppk2);
    }

    /// Disables the power on the ppk2 device
    /// This does not have an immediate effect on the target board
    pub fn power_disable(&mut self) {
        let mut ppk2 = self.take();
        ppk2.set_device_power(DevicePower::Disabled).unwrap();
        self.put(ppk2);
    }

    fn take(&mut self) -> Ppk2 {
        self.ppk2.take().unwrap()
    }

    fn put(&mut self, ppk2: Ppk2) {
        self.ppk2 = Some(ppk2);
    }

    fn stop_and_put(&mut self, stop_ppk2: impl FnOnce() -> Result<Ppk2, ppk2::Error>) {
        self.put(stop_ppk2().unwrap());
    }

    fn print_header() {
        println!("\n  Section  | µs/Sample | Current (µA)     | Status");
        println!("=======================================================");
    }

    fn print_status(
        status: &MeasureStatus,
        Sample {
            timestamp,
            duration,
            current,
            pins,
        }: &Sample,
    ) {
        let spinner = ['|', '/', '-', '\\'];
        print!(
            "\r| {:<8} | {:<9} | {:<16} | {:<9} | [{}]",
            pins.to_string(),
            duration.as_micros(),
            current.as_micros(),
            format!("{:?}", status),
            spinner[timestamp.as_secs() as usize % spinner.len()]
        );
        io::stdout().flush().unwrap();
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

#[derive(Debug, Clone)]
/// Predicate for when a measurement should be started or ended
pub enum When {
    /// Always evaluates to true
    Now,
    /// Always evaluates to false
    Never,
    /// An amount of time has elapsed
    Time(Duration),
    /// A mark has been identified via a pin configuration
    Logic(Pins),
    /// The Current is greater than a value
    CurrentGt(Current),
    /// The Current is less than a value
    CurrentLt(Current),
    /// Negates the predicate
    Not(Box<When>),
    /// Logical AND
    And(Box<When>, Box<When>),
    /// Logical OR
    Or(Box<When>, Box<When>),
    /// Logical XOR
    Xor(Box<When>, Box<When>),
}

impl When {
    /// Evaluates the predicate from the information given.
    pub fn eval(
        &self,
        sample @ Sample {
            timestamp,
            duration: _,
            current,
            pins,
        }: &Sample,
    ) -> bool {
        use When::*;
        match self {
            Now => true,
            Never => false,
            Time(pred_timestamp) => timestamp > pred_timestamp,
            Logic(pred_pins) => pins == pred_pins,
            CurrentGt(pred_current) => current > pred_current,
            CurrentLt(pred_current) => current < pred_current,
            Not(pred) => !pred.eval(sample),
            And(left, right) => left.eval(sample) && right.eval(sample),
            Or(left, right) => left.eval(sample) || right.eval(sample),
            Xor(left, right) => left.eval(sample) ^ right.eval(sample),
        }
    }
}

/// The status of the measurement.
///
/// Used in [Setup::measure] to keep track if we are waiting for a predicate to
/// start measuring or if we are measuring until a predicate holds to stop the
/// measurement.
#[derive(Debug, Clone)]
enum MeasureStatus {
    Waiting,
    Measuring,
}
