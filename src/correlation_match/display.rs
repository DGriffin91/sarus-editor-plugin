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

//! Wrapper for [`CorrelationMatch`], providing functionality for displaying a waveform.

use std::f32::consts::PI;

use super::iter_windows::{shift_left, shift_left_fill, shift_right, shift_right_fill};
use crate::correlation_match::CorrelationMatch;

/// Stores a prepared [`CorrelationMatch`] and buffers for display and memory.
pub struct DisplayBuffer {
    size: usize,
    correlation_matcher: CorrelationMatch,
    buffer: Vec<f32>,
    display: Vec<f32>,
    memory: Vec<f32>,
    weight: Vec<f32>,
    offset: usize,
    residual: f32,
    average_period: f32,
}

impl DisplayBuffer {
    /// Construct a new [`DisplayBuffer`] with given input buffer size and display
    /// buffer size.
    ///
    /// `input_size` must be at least as learge as `display_size`.
    ///
    /// The weight function is populated with a Hann window, so the center of the display is
    /// prioritized when matching.
    pub fn new(input_size: usize, display_size: usize) -> Self {
        assert!(input_size >= display_size);
        let weight = (0..display_size)
            .map(|index| index as isize - (display_size / 2) as isize)
            .map(|offset| offset as f32 / display_size as f32)
            .map(|x| 1. + (2. * PI * x).cos())
            .collect();
        DisplayBuffer {
            size: display_size,
            correlation_matcher: CorrelationMatch::new(input_size),
            buffer: vec![0.; input_size],
            display: vec![0.; display_size],
            memory: vec![0.; display_size],
            weight,
            offset: 0,
            residual: 0.,
            average_period: 0.,
        }
    }

    /// Scroll all internal buffers by the given signed amount of samples, to the right.
    ///
    /// Missing data is retrieved from the input buffer, or replaced with zeros if not available.
    pub fn scroll(&mut self, amount: i32) {
        match amount {
            amount if amount > 0 => {
                let amount = amount as usize;
                shift_right_fill(&mut self.buffer, amount, 0.);
                let replace_range = &self.buffer[self.offset..][..amount];
                shift_right(&mut self.display, replace_range);
                shift_right(&mut self.memory, replace_range);
            }
            amount if amount < 0 => {
                let amount = -amount as usize;
                shift_left_fill(&mut self.buffer, amount, 0.);
                let replace_range = &self.buffer[self.offset + self.size - amount..][..amount];
                shift_left(&mut self.display, replace_range);
                shift_left(&mut self.memory, replace_range);
            }
            _ => {}
        }
    }

    /// Get a mutable reference to the input buffer. If it is mutated, remember to call
    /// [`update_match`](Self::update_match) afterwards.
    ///
    /// The length of the slice is `input_size` given on construction.
    pub fn get_buffer_mut(&mut self) -> &mut [f32] {
        &mut self.buffer
    }

    /// Update the correlation match position, memory buffer and period estimate based on newest data.
    ///
    /// If `stabilize` is set to `false`, no matching is performed, and the previously set offset
    /// is retained.
    ///
    /// The `memory_decay` and `period_decay` parameters determine the decay coefficient of the
    /// memory buffer, and the interval average respectively:  
    /// ```none
    /// average = coeff * new + (1. - coeff) * average;
    /// ```
    /// Set to `1.0` to bypass smoothing.
    ///
    /// [`update_display`](Self::update_display) should be called separately to update the display buffer.
    pub fn update_match(&mut self, stabilize: bool, memory_decay: f32, period_decay: f32) {
        if stabilize {
            let (offset, interval) =
                self.correlation_matcher
                    .compute(&self.buffer, &self.memory, &self.weight);
            let rounded = offset.round();
            self.offset = rounded as usize;
            self.residual += offset - rounded;
            self.offset = (self.offset as i64 + self.residual as i64)
                .clamp(0, self.buffer.len() as i64 - 1) as usize;
            self.residual = self.residual.fract();
            if let Some(interval) = interval {
                self.average_period =
                    period_decay * interval + (1. - period_decay) * self.average_period;
            }
        }
        for (index, item) in self.memory.iter_mut().enumerate() {
            *item = memory_decay * self.buffer[index + self.offset] + (1. - memory_decay) * *item;
        }
    }

    /// Update the display buffer based on the newest input data and matched offset.
    ///
    /// This method may be called more often than [`update_match`](Self::update_match), even when
    /// there is no new data, to animate smoothly.
    pub fn update_display(&mut self, display_decay: f32) {
        for (index, item) in self.display.iter_mut().enumerate() {
            *item = display_decay * self.buffer[index + self.offset] + (1. - display_decay) * *item;
        }
    }

    /// Retrieve the contents of the display buffer.
    ///
    /// The length of the slice is `display_size` given on construction.
    pub fn get_display(&self) -> &[f32] {
        &self.display
    }

    /// Retrieve the contents of the memory buffer. This is what is used to find a
    /// match in [`update_match`](Self::update_match).
    ///
    /// The length of the slice is `display_size` given on construction.
    pub fn get_memory(&self) -> &[f32] {
        &self.memory
    }

    /// Get current estimated period.
    ///
    /// The fundamental frequency may be obtained via the sampling rate as follows:
    /// ```none
    /// f = SAMPLE_RATE / period
    /// ```
    pub fn get_period(&self) -> f32 {
        self.average_period
    }

    /// Get the current offset and residual.
    ///
    /// The first item of the tuple is a whole f32ber, denoting the starting index of the latest
    /// match in the input buffer. The second item denotes accumulated subsample precision, which
    /// is less than `1` by absolute value. A plot of the waveform should be offset by the negation
    /// of the residual.
    pub fn get_offset(&self) -> (usize, f32) {
        (self.offset, self.residual)
    }
}
