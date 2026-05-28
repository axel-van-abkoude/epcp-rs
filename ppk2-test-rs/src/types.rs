//! Types that are used to store power measurements and metrics

use std::{
    array::from_fn,
    fmt::{self, Display, Formatter},
    iter::zip,
    ops::{Add, AddAssign, Index, IndexMut},
    time::Duration,
};

use ppk2::{
    measurement::Measurement,
    types::{Level, LogicPortPins},
};
use serde::{
    Deserialize, Serialize,
    de::{Deserializer, Error, Visitor},
    ser::Serializer,
};

macro_rules! write_arg {
    ($f:expr, $label:expr, $arg:expr) => {{
        write!(
            $f,
            "{}:{:>width$}",
            $label,
            $arg,
            width = $f.width().unwrap_or(30) - $label.chars().count() - 1,
        )
    }};
}

/// The data associated with a section
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Section {
    /// The total time spent in a section in the total timespan
    pub pins: Pins,
    /// The total time spent in a section in the total timespan
    pub total_duration: Duration,
    /// The total capacity of a section in the total timespan
    pub total_capacity: Capacity,
}

impl From<Pins> for Section {
    fn from(value: Pins) -> Self {
        Section {
            pins: value,
            total_duration: Duration::ZERO,
            total_capacity: Capacity::ZERO,
        }
    }
}

impl Display for Section {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write_arg!(
            f,
            "| section",
            String::new() + "(" + &u8::from(self.pins).to_string() + ") " + &self.pins.to_string()
        )?;
        write_arg!(f, " | µs", self.total_duration.as_micros())?;
        write_arg!(f, " | µAh", self.total_capacity.as_micros())?;
        writeln!(f, "|")?;
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
/// The datatype that stores all sections
pub struct Sections([Section; 256]);

impl Display for Sections {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", "=".repeat(91))?;
        for section in self.0.iter() {
            match *section {
                Section {
                    pins: _,
                    total_duration: Duration::ZERO,
                    total_capacity: Capacity::ZERO,
                } => continue,
                s => {
                    write!(f, "{}", s)?;
                }
            }
        }
        writeln!(f, "|{}|", "-".repeat(89))?;
        write_arg!(f, "| Total Measurement", "")?;
        write_arg!(f, " | µs", self.total_duration().as_micros())?;
        write_arg!(f, " | µAh", self.total_capacity())?;
        writeln!(f, "|\n{}", "=".repeat(91))?;
        Ok(())
    }
}

impl Sections {
    /// Initializes all sections with the index mapped to a section
    pub fn new() -> Sections {
        Sections(from_fn(|i| Section::from(Pins::from(i as u8))))
    }

    /// Returns the total capacity of all sections combined
    pub fn total_capacity(mut self) -> Capacity {
        self.0
            .iter_mut()
            .reduce(|acc, section| {
                acc.total_capacity += section.total_capacity;
                acc
            })
            .unwrap()
            .total_capacity
    }

    /// Returns the total duration of all sections combined
    pub fn total_duration(mut self) -> Duration {
        self.0
            .iter_mut()
            .reduce(|acc, section| {
                acc.total_duration += section.total_duration;
                acc
            })
            .unwrap()
            .total_duration
    }

    /// Update the Sections with a measurement and the duration of that measurement
    pub fn update_with(&mut self, measurement: Measurement, duration: Duration) {
        let section = &mut self[Pins::from(measurement.pins)];

        // µA * µs = A*s*(µ^2)
        // h:  sec / 60^2
        section.total_capacity += measurement.micro_amps * (duration.as_micros() as f32);
        section.total_duration += duration;
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
/// Capacity in µA
pub struct Capacity(i64);

impl Display for Capacity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Add for Capacity {
    type Output = Capacity;
    fn add(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 + rhs.0;
        self
    }
}

impl AddAssign<Capacity> for Capacity {
    fn add_assign(&mut self, rhs: Capacity) {
        *self = *self + rhs;
    }
}

impl AddAssign<f32> for Capacity {
    fn add_assign(&mut self, rhs: f32) {
        self.0 += rhs as i64;
    }
}

impl Capacity {
    /// Zero capacity
    pub const ZERO: Capacity = Capacity(0i64);

    /// 500 µA
    pub const MICROS_500: Capacity = Capacity(500i64);

    /// Create a capacity from micro_amps
    pub fn from_micros(micros: f32) -> Self {
        Self(micros as i64)
    }

    /// Capacity returned as µA
    pub fn as_micros(self) -> i64 {
        self.0
    }
}

#[derive(Copy, Debug, Clone)]
/// Implementation of ppk2-rs LogicPortPins
pub struct Pins(LogicPortPins);

impl From<LogicPortPins> for Pins {
    fn from(value: LogicPortPins) -> Self {
        Self(value)
    }
}

impl PartialEq for Pins {
    fn eq(&self, other: &Self) -> bool {
        for (inner_self, inner_other) in zip(self.0.inner(), other.0.inner()) {
            if PinLevel(*inner_self) != PinLevel(*inner_other) {
                return false;
            }
        }
        return true;
    }
}

impl IndexMut<Pins> for Sections {
    fn index_mut(&mut self, index: Pins) -> &mut Self::Output {
        &mut self.0[u8::from(index) as usize]
    }
}

impl Index<Pins> for Sections {
    type Output = Section;
    fn index(&self, index: Pins) -> &Self::Output {
        &self.0[u8::from(index) as usize]
    }
}

impl From<Pins> for u8 {
    fn from(value: Pins) -> Self {
        let mut ret: u8 = 0;
        for (i, pin) in value.0.inner().iter().enumerate() {
            match pin {
                Level::Low => {
                    continue;
                }
                Level::High => {
                    ret |= 1 << i;
                }
                Level::Either => {
                    todo!("either in u8::from");
                }
            }
        }
        ret
    }
}

impl From<u8> for Pins {
    fn from(value: u8) -> Self {
        Pins(LogicPortPins::from(value))
    }
}

impl ToString for Pins {
    fn to_string(&self) -> String {
        self.0
            .inner()
            .iter()
            .map(|p| char::from(PinLevel(*p)))
            .collect()
    }
}

/// Serializer for the string representation a section
impl Serialize for Pins {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Serializer for the string representation of a section
impl<'de> Deserialize<'de> for Pins {
    fn deserialize<D>(deserializer: D) -> Result<Pins, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(PinsVisitor)
    }
}

struct PinsVisitor;
impl<'de> Visitor<'de> for PinsVisitor {
    type Value = Pins;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("an string representation of the logic port pins (111100xx -> high high high high low low either either)")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if v.as_bytes().len() != 8 {
            return Err(E::custom("length of pins != 8"));
        }
        let mut arr = [Level::Either; 8];
        for i in 0..8 {
            arr[i] = PinLevel::from(v.as_bytes()[i] as char).0;
        }
        Ok(Pins(LogicPortPins::with_levels(arr)))
    }
}

#[derive(Debug, Clone)]
/// Implementation of ppk2-rs Level
pub struct PinLevel(Level);

impl PartialEq for PinLevel {
    fn eq(&self, other: &Self) -> bool {
        match (self.0, other.0) {
            (Level::Low, Level::Low) => true,
            (Level::High, Level::High) => true,

            // Can be used for queries on sections
            (Level::Either, _) => true,
            (_, Level::Either) => true,

            _ => false,
        }
    }
}

impl From<Level> for PinLevel {
    fn from(value: Level) -> Self {
        Self(value)
    }
}

/// Implementation to parse a PinLevel from a char used in deserialisation
impl From<char> for PinLevel {
    fn from(c: char) -> Self {
        match c {
            '0' => PinLevel(Level::Low),
            '1' => PinLevel(Level::High),
            _ => PinLevel(Level::Either),
        }
    }
}

/// Implementation to convert PinLevel to a char used in serialisation
impl From<PinLevel> for char {
    fn from(level: PinLevel) -> Self {
        match level.0 {
            Level::Low => '0',
            Level::High => '1',
            Level::Either => 'x',
        }
    }
}

/// One Sample collected by the ppk2 containing:
/// * the timestamp in the measurement
/// * the duration of the sample itself
/// * the average current of the sample
/// * the most prevalent pin configuration of the sample
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Sample {
    #[serde(rename = "Timestamp In Measurement (μs)")]
    #[serde(serialize_with = "ser_duration_micros")]
    #[allow(missing_docs)]
    pub timestamp: Duration,
    #[serde(rename = "Duration Sample (μs)")]
    #[serde(serialize_with = "ser_duration_micros")]
    #[allow(missing_docs)]
    pub duration: Duration,
    #[serde(rename = "Current Sample (μA)")]
    #[allow(missing_docs)]
    pub current: Current,
    #[serde(rename = "Logic Pins Sample (D0-D7)")]
    #[allow(missing_docs)]
    pub pins: Pins,
}

fn ser_duration_micros<S>(d: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_u64(d.as_micros() as u64)
}

/// Current in Ampere stored as μA
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Current(f32);

impl Current {
    #[allow(missing_docs)]
    pub fn from_micros(micros: f32) -> Self {
        Self(micros)
    }

    #[allow(missing_docs)]
    pub fn as_micros(&self) -> f32 {
        self.0
    }
}

impl PartialOrd for Current {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inverses_level() {
        assert_eq!(
            PinLevel(Level::Low),
            PinLevel::from(char::from(PinLevel(Level::Low)))
        );
        assert_eq!(
            PinLevel(Level::High),
            PinLevel::from(char::from(PinLevel(Level::High)))
        );
        assert_eq!(
            PinLevel(Level::Either),
            PinLevel::from(char::from(PinLevel(Level::Either)))
        );
    }

    #[test]
    fn test_inverses_char() {
        assert_eq!('0', char::from(PinLevel::from('0')));
        assert_eq!('1', char::from(PinLevel::from('1')));
        assert_eq!('x', char::from(PinLevel::from('x')));
        assert_eq!('x', char::from(PinLevel::from('\x00')));
    }

    #[test]
    fn test_inverses_u8() {
        for i in 0..256 {
            assert_eq!(i as u8, u8::from(Pins::from(i as u8)));
        }
    }
}
