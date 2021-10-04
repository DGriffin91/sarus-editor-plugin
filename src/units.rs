use ringbuf::Consumer;

pub struct ConsumerDump<T> {
    pub data: Vec<T>,
    consumer: Consumer<T>,
    max_size: usize,
}

impl<T> ConsumerDump<T> {
    pub fn new(consumer: Consumer<T>, max_size: usize) -> ConsumerDump<T> {
        ConsumerDump {
            data: Vec::new(),
            consumer,
            max_size,
        }
    }

    pub fn consume(&mut self) {
        for _ in 0..self.consumer.len() {
            if let Some(n) = self.consumer.pop() {
                self.data.push(n);
            } else {
                break;
            }
        }
        self.trim_data()
    }

    pub fn set_max_size(&mut self, max_size: usize) {
        self.max_size = max_size;
        self.trim_data();
    }

    pub fn trim_data(&mut self) {
        //Trims from the start of the vec
        let data_len = self.data.len();
        if data_len > self.max_size {
            self.data.drain(0..(data_len - self.max_size).max(0));
        }
    }
}
