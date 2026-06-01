//! Logic that gives context to measurements or instructs what is measured

use std::fmt::{self, Formatter};

use crate::{data::Sample, unit::Current};
use ppk2::types::{Level, LogicPortPins};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error, Visitor},
};
use std::iter::zip;
use std::time::Duration;

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
pub enum MeasureStatus {
    /// Indicates that we are waiting for a condition to hold
    Waiting,
    /// Indicates that a measurement is taking place
    Measuring,
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
