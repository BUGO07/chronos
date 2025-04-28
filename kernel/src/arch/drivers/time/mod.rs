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
    tsc::init();
    TIMERS_INIT_STATE.store(2, Ordering::Relaxed);
    kvm::init();
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

pub fn preferred_timer_ns() -> u64 {
    let state = TIMERS_INIT_STATE.load(Ordering::Relaxed);
    let pit = PIT_MS.load(Ordering::Relaxed) * 1_000_000;
    let kvm = unsafe { crate::arch::drivers::time::kvm::KVM_TIMER.get() };
    let tsc = unsafe { crate::arch::drivers::time::tsc::TSC_TIMER.get() };
    let hpet = unsafe { crate::arch::drivers::time::hpet::HPET_TIMER.get() };

    match state {
        4..=5 => kvm
            .map(|t| t.elapsed_ns())
            .or_else(|| tsc.map(|t| t.elapsed_ns()))
            .or_else(|| hpet.map(|t| t.elapsed_ns()))
            .unwrap_or(pit),
        2..=3 => kvm
            .filter(|t| t.is_supported())
            .map(|t| t.elapsed_ns())
            .or_else(|| tsc.filter(|t| t.is_supported()).map(|t| t.elapsed_ns()))
            .unwrap_or(pit),
        _ => pit,
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
