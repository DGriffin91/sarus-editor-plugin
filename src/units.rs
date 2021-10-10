use ringbuf::Consumer;

pub struct ConsumerRingBuf<T> {
    pub data: Vec<T>,
    consumer: Consumer<T>,
    idx: usize,
}

impl<T> ConsumerRingBuf<T>
where
    T: std::default::Default + std::clone::Clone,
{
    pub fn new(consumer: Consumer<T>, max_size: usize) -> ConsumerRingBuf<T> {
        ConsumerRingBuf {
            data: vec![T::default(); max_size],
            consumer,
            idx: 0,
        }
    }

    pub fn consume(&mut self) {
        for _ in 0..self.consumer.len() {
            if let Some(n) = self.consumer.pop() {
                self.data[self.idx] = n;
                self.idx += 1;
                if self.idx >= self.data.len() {
                    self.idx = 0
                }
            } else {
                break;
            }
        }
    }

    pub fn iter(&self) -> std::iter::Chain<std::slice::Iter<'_, T>, std::slice::Iter<'_, T>> {
        self.data[self.idx..]
            .iter()
            .chain(self.data[0..self.idx].iter())
    }
}
