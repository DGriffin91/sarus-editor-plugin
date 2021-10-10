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

const EPS: f64 = 1e-8;

/// Finds the approximate minimum point of a function given three fixed points,
/// using a parabola.
///
/// Formulas derived in `/dokumentaatio/parabolic_interpolation_formulas.py`
///
/// Given three points, `(0, a), (1, b), (2, c)`, finds the approximate minimum point. Returns a
/// pair `(x, y)`, describing that point. May also return None, if there is no minimum point or it
/// is not on the interval [0, 2].
#[inline]
pub fn parabolic_interpolation_minimum(a: f64, b: f64, c: f64) -> Option<(f64, f64)> {
    // x^2 coefficient should be positive: parabola opens upwards
    let x2coefficient = 2. * (a - 2. * b + c);
    if x2coefficient > EPS {
        let v = 3. * a - 4. * b + c;
        let position = v / x2coefficient;
        if (0. ..=2.).contains(&position) {
            let value = a - v * position / 4.;
            Some((position, value))
        } else {
            None
        }
    } else {
        None
    }
}
