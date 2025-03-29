use spin::Mutex;
use x86_64::structures::idt::InterruptStackFrame;

pub const PIT_FREQUENCY: u32 = 1193182;

pub struct Time(pub f64);

pub static TIME: Mutex<Time> = Mutex::new(Time(0.0));

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

pub extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    TIME.lock().increment(PIT_FREQUENCY as f64 / 100_000_000.0);
    super::drivers::pic::send_eoi();
}

pub fn print_time() {
    crate::println!("{}", super::time::TIME.lock().get())
}
