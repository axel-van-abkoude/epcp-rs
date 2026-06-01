//! Types that store SI units which ensures we keep data correct through conversions.

use std::{
    fmt::{self, Display, Formatter},
    ops::{Add, AddAssign},
};

use serde::{Deserialize, Serialize};

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
