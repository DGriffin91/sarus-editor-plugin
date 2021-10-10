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

//! Ring buffer for transferring audio data between threads in a wait-free manner.
//!
//! Functions as a single-producer single-consumer FIFO queue, where whole slices can be
//! pushed and read at once.
//!
//! All data is stored in one contiguous array, where `read_index` and `write_index`
//! wrap around to the beginning if they reach the end. That is why it is called
//! a "ring".

use std::cell::UnsafeCell;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// Defines the interface that an item stored in the queue must implement. Only very simple types,
/// such as floating point numbers or small `struct`s that implement Copy are permitted.
pub trait Item: Copy + Send + Sync + Default {}
impl<T: Copy + Send + Sync + Default> Item for T {}

struct RingBuffer<T: Item> {
    capacity: usize,
    buffer: Vec<UnsafeCell<T>>,
    read_index: AtomicUsize,
    write_index: AtomicUsize,
}

// SAFETY: Only a corresponding pair of producer and consumer can be constructed, and they cannot
// be cloned. No element of the buffer is ever accessed concurrently by the producer and
// the consumer.
unsafe impl<T: Item> Sync for RingBuffer<T> {}

/// The Producer party of a ring buffer. Can only push data to the queue.
pub struct Producer<T: Item> {
    internal: Arc<RingBuffer<T>>,
}

/// The Consumer party of a ring buffer. Can only pop data off the queue.
pub struct Consumer<T: Item> {
    internal: Arc<RingBuffer<T>>,
}

struct RingRange {
    start: usize,
    open_end: usize,
    capacity: usize,
}

/// Construct a ring buffer with a given capacity, and return `(producer, consumer)`.
/// The producer can then be sent to another thread for example.
///
/// Internally, an allocation that is one larger than the given capacity is made.
/// This is because for an allocation of size `capacity + 1`, there are exactly `capacity + 1`
/// different offsets the read and write indices can point to, which corresponds to the range
/// `0..=capacity`.
pub fn with_capacity<T: Item>(capacity: usize) -> (Producer<T>, Consumer<T>) {
    let capacity = capacity + 1;
    let ring_buffer = Arc::new(RingBuffer {
        capacity,
        buffer: (0..capacity).map(|_| Default::default()).collect(),
        read_index: AtomicUsize::new(0),
        write_index: AtomicUsize::new(0),
    });
    (
        Producer {
            internal: ring_buffer.clone(),
        },
        Consumer {
            internal: ring_buffer,
        },
    )
}

impl<T: Item> RingBuffer<T> {
    fn produce_available(&self) -> RingRange {
        let read_index = self.read_index.load(Ordering::Relaxed);
        let write_index = self.write_index.load(Ordering::Relaxed);
        // Offer one less than allocated capacity
        let write_limit = if read_index == 0 {
            self.capacity - 1
        } else {
            read_index - 1
        };
        RingRange {
            start: write_index,
            open_end: write_limit,
            capacity: self.capacity,
        }
    }

    fn consume_available(&self) -> RingRange {
        let read_index = self.read_index.load(Ordering::Relaxed);
        // Ordering::Acquire to ensure that data written to the buffer is visible
        // to this consumer thread. Corresponding Release is in commit_produced.
        let write_index = self.write_index.load(Ordering::Acquire);
        RingRange {
            start: read_index,
            open_end: write_index,
            capacity: self.capacity,
        }
    }

    fn commit_produced(&self, amount: usize) {
        let mut write_index = self.write_index.load(Ordering::Relaxed);
        write_index += amount;
        if write_index >= self.capacity {
            write_index -= self.capacity;
        }
        self.write_index.store(write_index, Ordering::Release);
    }

    fn commit_consumed(&self, amount: usize) {
        let mut read_index = self.read_index.load(Ordering::Relaxed);
        read_index += amount;
        if read_index >= self.capacity {
            read_index -= self.capacity;
        }
        self.read_index.store(read_index, Ordering::Relaxed);
    }
}

impl<T: Item> Producer<T> {
    /// Tries to push the contents of the given slice into the buffer. If only part
    /// of it fits in the buffer, `Err(n)` will be returned, where `n` is the amount of
    /// items that were copied into the buffer.
    pub fn push(&mut self, data: &[T]) -> Result<(), usize> {
        let range = self.internal.produce_available();
        let amount = data.len().min(range.size_hint().0);
        for (index, value) in range.zip(data.iter()) {
            // SAFETY: The ring buffer read & write indices guarantee the consumer
            // is not currently reading this index of the buffer, so no other references
            // to it exist. We have a unique mutable reference to self.
            unsafe {
                let slot = &mut *self.internal.buffer[index].get();
                *slot = *value;
            }
        }
        self.internal.commit_produced(amount);
        if amount < data.len() {
            Err(amount)
        } else {
            Ok(())
        }
    }
}

impl<T: Item> Consumer<T> {
    /// Tries to fill the entirety of the given `data` slice. If there is not enough data
    /// available, `Err(n)` is returned, where `n < data.len()` is the amount of items available.
    pub fn pop_full(&mut self, data: &mut [T]) -> Result<(), usize> {
        let range = self.internal.consume_available();
        let amount = data.len().min(range.size_hint().0);
        if amount < data.len() {
            Err(amount)
        } else {
            for (index, value) in range.zip(data.iter_mut()) {
                // SAFETY: The ring buffer read & write indices guarantee the producer
                // is not currently writing to this index of the buffer, so no other references to
                // it exist.
                unsafe {
                    let copy = *self.internal.buffer[index].get();
                    *value = copy;
                }
            }
            self.internal.commit_consumed(amount);
            Ok(())
        }
    }

    /// Discard all currently available data.
    pub fn discard_all(&mut self) {
        let range = self.internal.consume_available();
        let amount = range.size_hint().0;
        self.internal.commit_consumed(amount);
    }
}

impl Iterator for RingRange {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        if self.start != self.open_end {
            let index = self.start;
            self.start += 1;
            if self.start == self.capacity {
                self.start = 0;
            }
            Some(index)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size;
        if self.start <= self.open_end {
            size = self.open_end - self.start;
        } else {
            size = self.capacity - self.start + self.open_end;
        }
        (size, Some(size))
    }
}
