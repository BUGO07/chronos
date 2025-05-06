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

pub mod generic;

pub static TIMERS_INIT_STATE: AtomicU8 = AtomicU8::new(0);

static mut SLEEPING_TASKS: Vec<(u64, Waker)> = Vec::new();

pub fn early_init() {
    generic::init();
    TIMERS_INIT_STATE.store(1, Ordering::Relaxed);
}

pub fn init() {}

pub fn preferred_timer_ms() -> u64 {
    preferred_timer_ns() / 1_000_000
}

pub trait KernelTimer {
    fn name(&self) -> &'static str;
    fn is_supported(&self) -> bool;
    fn priority(&self) -> u8;
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
    unsafe {
        let generic = crate::arch::drivers::time::generic::GENERIC_TIMER
            .get()
            .map(|t| t as &dyn KernelTimer);
        let timers: [&Option<&dyn KernelTimer>; 1] = [&generic];

        for timer_opt in timers {
            if let Some(timer) = *timer_opt {
                if timer.is_supported() {
                    return timer.elapsed_ns();
                }
            }
        }

        0
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
            wake_at: preferred_timer_ms() + duration_ticks,
        }
    }
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if preferred_timer_ms() >= self.wake_at {
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
    let now = preferred_timer_ms();
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
