use rand::thread_rng;
use rand::Rng;
use std::fmt::Display;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::atomic_f32::AtomicF32;

//TODO get byte load/save to work in vsts so this is not needed

#[derive(Debug, Clone)]
pub struct FloatId {
    f1: Arc<AtomicF32>,
    f2: Arc<AtomicF32>,
}

impl FloatId {
    pub fn to_string(&self) -> String {
        format!("{}", self.get_u64())
    }
    pub fn from_str(s: &str) -> Self {
        FloatId::from_u64(s.parse::<u64>().unwrap())
    }
    pub fn from_f32s(f1: f32, f2: f32) -> Self {
        if f1 >= 10000000.0 {
            panic!("Max f32 for float id is 9999999. Found {}", f1)
        }
        if f2 >= 10000000.0 {
            panic!("Max f32 for float id is 9999999. Found {}", f2)
        }
        FloatId {
            f1: Arc::new(AtomicF32::new(f1.max(0.0).trunc() + 0.4)),
            f2: Arc::new(AtomicF32::new(f2.max(0.0).trunc() + 0.4)),
        }
    }
    pub fn update_from_f32(&self, f1: f32, f2: f32) {
        self.f1.set(f1);
        self.f2.set(f2);
    }
    pub fn update_from_u64(&self, n: u64) {
        let f = Self::f32_from_u64(n);
        self.f1.set(f.0);
        self.f2.set(f.1);
    }
    pub fn get_u64(&self) -> u64 {
        100000000000000 + self.f1.get() as u64 * 10000000 + self.f2.get() as u64
    }
    pub fn get_f32(&self) -> (f32, f32) {
        (self.f1.get(), self.f2.get())
    }
    pub fn from_u64(n: u64) -> Self {
        let f = Self::f32_from_u64(n);
        FloatId {
            f1: Arc::new(AtomicF32::new(f.0)),
            f2: Arc::new(AtomicF32::new(f.1)),
        }
    }
    pub fn f32_from_u64(n: u64) -> (f32, f32) {
        if n >= 200000000000000 {
            panic!("Max u64 for float id is 199999999999999. Found {}", n)
        } else if n <= 100000000000000 {
            panic!("Min u64 for float id is 100000000000001. Found {}", n)
        }
        let n = n - 100000000000000;
        let top = n / 10000000;
        let f1 = top as f32 + 0.4;
        let f2 = (n - (top * 10000000)) as f32 + 0.4;
        (f1, f2)
    }
    pub fn new() -> Self {
        let mut rand_num1 = (thread_rng().gen_range(0..999999) as u64) as f64;
        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let since_the_epoch = ((since_the_epoch as f64) * 0.00000001).fract() * 10000000.0;
        let n = since_the_epoch.trunc();
        rand_num1 += (since_the_epoch.fract() * 10.0).trunc() * 1000000.0;

        FloatId {
            f1: Arc::new(AtomicF32::new(n as f32 + 0.4)),
            f2: Arc::new(AtomicF32::new(rand_num1 as f32 + 0.4)),
        }
    }
}

impl Display for FloatId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    #[test]
    fn test_float_id() -> anyhow::Result<()> {
        let mut ids = HashSet::new();
        for _ in 0..100 {
            let id = FloatId::new();
            let s = id.to_string();
            if ids.contains(&s) {
                panic!("id {} already exists", s)
            }
            ids.insert(s);
            let id2 = FloatId::from_f32s(id.f1.get() + 0.05, id.f2.get() - 0.05);
            let id3 = FloatId::from_str(&id.to_string());
            let id4 = FloatId::from_u64(id.get_u64());
            assert_eq!(id.to_string(), id2.to_string());
            assert_eq!(id.to_string(), id3.to_string());
            assert_eq!(id.to_string(), id4.to_string());
        }

        Ok(())
    }
}
