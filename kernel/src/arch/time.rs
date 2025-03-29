pub const ONE_MS: u32 = 1193;

pub struct Time(pub f64);

impl Time {
    pub fn get(&self) -> f64 {
        self.0
    }
    pub fn set(&mut self, value: f64) {
        self.0 = value
    }
    pub fn increment(&mut self, value: f64) {
        self.0 += value;
    }
    pub fn decrement(&mut self, value: f64) {
        self.0 += value;
    }
}
