/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    pin::Pin,
    sync::atomic::{AtomicU8, Ordering},
    task::{Context, Poll, Waker},
};

use alloc::{format, string::String, vec::Vec};
use pit::PIT_MS;

pub mod hpet;
pub mod kvm;
pub mod pit;
pub mod rtc;
pub mod tsc;

pub static TIMERS_INIT_STATE: AtomicU8 = AtomicU8::new(0);

static mut SLEEPING_TASKS: Vec<(u64, Waker)> = Vec::new();

pub fn early_init() {
    pit::init();
    TIMERS_INIT_STATE.store(1, Ordering::Relaxed);
    kvm::init();
    TIMERS_INIT_STATE.store(2, Ordering::Relaxed);
    tsc::init();
    TIMERS_INIT_STATE.store(3, Ordering::Relaxed);
}

pub fn init() {
    hpet::init();
    TIMERS_INIT_STATE.store(4, Ordering::Relaxed);
    crate::arch::system::lapic::init();
    TIMERS_INIT_STATE.store(5, Ordering::Relaxed);
}

pub fn preferred_timer_ms() -> u64 {
    preferred_timer_ns() / 1_000_000
}

pub trait KernelTimer {
    fn name(&self) -> &'static str;
    fn is_supported(&self) -> bool;
    fn elapsed_ns(&self) -> u64;

    fn elapsed_pretty(&self, digits: u32) -> String {
        let elapsed_ns = self.elapsed_ns();
        let subsecond_ns = elapsed_ns % 1_000_000_000;

        let divisor = 10u64.pow(9 - digits);
        let subsecond = subsecond_ns / divisor;

        let elapsed_ms = elapsed_ns / 1_000_000;
        let seconds_total = elapsed_ms / 1000;
        let seconds = seconds_total % 60;
        let minutes_total = seconds_total / 60;
        let minutes = minutes_total % 60;
        let hours = minutes_total / 60;

        format!(
            "{:02}:{:02}:{:02}.{:0width$}",
            hours,
            minutes,
            seconds,
            subsecond,
            width = digits as usize
        )
    }
}

pub fn preferred_timer_ns() -> u64 {
    let state = TIMERS_INIT_STATE.load(Ordering::Relaxed);
    let pit = PIT_MS.load(Ordering::Relaxed) * 1_000_000;

    unsafe {
        let kvm = crate::arch::drivers::time::kvm::KVM_TIMER
            .get()
            .map(|t| t as &dyn KernelTimer);
        let tsc = crate::arch::drivers::time::tsc::TSC_TIMER
            .get()
            .map(|t| t as &dyn KernelTimer);
        let hpet = crate::arch::drivers::time::hpet::HPET_TIMER
            .get()
            .map(|t| t as &dyn KernelTimer);

        let timers: [&Option<&dyn KernelTimer>; 3] = match state {
            4..=5 => [&kvm, &tsc, &hpet],
            2..=3 => [&kvm, &tsc, &None],
            _ => [&None, &None, &None],
        };

        for timer_opt in timers {
            if let Some(timer) = *timer_opt {
                if timer.is_supported() {
                    return timer.elapsed_ns();
                }
            }
        }

        pit
    }
}

pub fn preferred_timer_pretty(digits: u32) -> String {
    let elapsed_ns = preferred_timer_ns();
    let subsecond_ns = elapsed_ns % 1_000_000_000;

    let divisor = 10u64.pow(9 - digits);
    let subsecond = subsecond_ns / divisor;

    let elapsed_ms = elapsed_ns / 1_000_000;
    let seconds_total = elapsed_ms / 1000;
    let seconds = seconds_total % 60;
    let minutes_total = seconds_total / 60;
    let minutes = minutes_total % 60;
    let hours = minutes_total / 60;

    format!(
        "{:02}:{:02}:{:02}.{:0width$}",
        hours,
        minutes,
        seconds,
        subsecond,
        width = digits as usize
    )
}

pub struct TimerFuture {
    wake_at: u64,
}

impl TimerFuture {
    pub fn new(duration_ticks: u64) -> Self {
        TimerFuture {
            wake_at: preferred_timer_ms() as u64 + duration_ticks,
        }
    }
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if preferred_timer_ms() as u64 >= self.wake_at {
            Poll::Ready(())
        } else {
            register_sleep(self.wake_at, cx.waker().clone());
            Poll::Pending
        }
    }
}

pub fn register_sleep(wake_at: u64, waker: Waker) {
    unsafe { SLEEPING_TASKS.push((wake_at, waker)) };
}

pub fn wake_ready_tasks() {
    let now = preferred_timer_ms() as u64;
    let tasks = unsafe { &mut SLEEPING_TASKS };
    let mut i = 0;
    while i < tasks.len() {
        if tasks[i].0 <= now {
            let (_, waker) = tasks.remove(i);
            waker.wake();
        } else {
            i += 1;
        }
    }
}
