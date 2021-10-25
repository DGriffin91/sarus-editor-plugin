use rand::thread_rng;
use rand::Rng;
use std::fmt::Display;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

//TODO get byte load/save to work in vsts so this is not needed

pub struct FloatId {
    f1: f32,
    f2: f32,
    f3: f32,
}

impl FloatId {
    pub fn to_string(&self) -> String {
        format!(
            "{}{}{}",
            &format!("{:.7}", self.f1)[2..9],
            &format!("{:.7}", self.f2)[2..9],
            &format!("{:.7}", self.f3)[2..9],
        )
    }
    pub fn from_str(s: &str) -> Self {
        FloatId {
            f1: (s[0..7].parse::<u64>().unwrap() as f64 / 10000000.0 + 0.00000002) as f32,
            f2: (s[7..14].parse::<u64>().unwrap() as f64 / 10000000.0 + 0.00000002) as f32,
            f3: (s[14..21].parse::<u64>().unwrap() as f64 / 10000000.0 + 0.00000002) as f32,
        }
    }
    pub fn from_f32s(f1: f32, f2: f32, f3: f32) -> Self {
        FloatId { f1, f2, f3 }
    }
    pub fn new() -> Self {
        let rand_num1 = (thread_rng().gen_range(0..9999999) as u64) as f32 * 0.0000001;
        let rand_num2 = (thread_rng().gen_range(0..9999999) as u64) as f32 * 0.0000001;
        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let n = (((since_the_epoch as f64) / 10.0).trunc() / 10000000.0).fract() as f32;

        FloatId {
            f1: n + 0.00000002,
            f2: rand_num1 as f32 + 0.00000002,
            f3: rand_num2 as f32 + 0.00000002,
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
        for _ in 0..100000 {
            let id = FloatId::new();
            let s = id.to_string();
            if ids.contains(&s) {
                panic!("id {} already exists", s)
            }
            ids.insert(s);
            let id2 = FloatId::from_f32s(
                id.f1 + 0.000000005,
                id.f2 - 0.000000005,
                id.f3 + 0.000000005,
            );
            let id3 = FloatId::from_str(&id.to_string());
            assert_eq!(id.to_string(), id2.to_string());
            assert_eq!(id.to_string(), id3.to_string());
        }

        Ok(())
    }
}
