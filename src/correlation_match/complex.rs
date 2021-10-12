/*
MIT License

Copyright (c) 2021 Roope Salmi

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

use std::{
    fmt,
    ops::{Add, Div, Mul, Neg, Sub},
};

/// The imaginary unit "i".
pub const IMAG_UNIT: Complex = Complex { real: 0., imag: 1. };

/// A complex f32ber.
#[derive(Copy, Clone)]
pub struct Complex {
    pub real: f32,
    pub imag: f32,
}

impl Complex {
    /// The complex conjugate.
    pub fn conj(self) -> Self {
        Complex {
            real: self.real,
            imag: -self.imag,
        }
    }

    /// Square of the absolute value.
    pub fn abs2(self) -> f32 {
        self.real * self.real + self.imag * self.imag
    }

    /// Absolute value.
    pub fn abs(self) -> f32 {
        self.abs2().sqrt()
    }

    /// Euler's formula,
    /// `e^(ix) = cos x + i sin x`.
    pub fn euler(x: f32) -> Self {
        Complex {
            real: x.cos(),
            imag: x.sin(),
        }
    }
}

/// Defines a conversion from a pair of real f32bers into a complex f32ber.
impl From<(f32, f32)> for Complex {
    fn from(pair: (f32, f32)) -> Complex {
        Complex {
            real: pair.0,
            imag: pair.1,
        }
    }
}

/// Displays a complex f32ber in text form, akin to "(a + bi)".
impl fmt::Debug for Complex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.imag >= 0. {
            write!(f, "({} + {}i)", self.real, self.imag)
        } else {
            write!(f, "({} - {}i)", self.real, -self.imag)
        }
    }
}

// Basic mathematical operators are defined for complex f32bers below.

impl Add for Complex {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Complex {
            real: self.real + other.real,
            imag: self.imag + other.imag,
        }
    }
}

impl Sub for Complex {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Complex {
            real: self.real - other.real,
            imag: self.imag - other.imag,
        }
    }
}

impl Neg for Complex {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Complex {
            real: -self.real,
            imag: -self.imag,
        }
    }
}

impl Mul<f32> for Complex {
    type Output = Self;
    fn mul(self, other: f32) -> Self::Output {
        Complex {
            real: self.real * other,
            imag: self.imag * other,
        }
    }
}

impl Div<f32> for Complex {
    type Output = Self;
    fn div(self, other: f32) -> Self::Output {
        Complex {
            real: self.real / other,
            imag: self.imag / other,
        }
    }
}

impl Mul for Complex {
    type Output = Self;
    fn mul(self, other: Self) -> Self::Output {
        Complex {
            real: self.real * other.real - self.imag * other.imag,
            imag: self.real * other.imag + self.imag * other.real,
        }
    }
}

impl Div for Complex {
    type Output = Self;
    fn div(self, other: Self) -> Self::Output {
        let dividend = Complex {
            real: self.real * other.real + self.imag * other.imag,
            imag: self.imag * other.real - self.real * other.imag,
        };
        let divisor = other.abs2();
        dividend / divisor
    }
}
