/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    pin::Pin,
    sync::atomic::{AtomicU8, Ordering},
    task::{Context, Poll, Waker},
};

use alloc::{string::String, vec::Vec};

use crate::utils::time::KernelTimer;

pub mod generic;

pub static TIMERS_INIT_STATE: AtomicU8 = AtomicU8::new(0);

static mut SLEEPING_TASKS: Vec<(u64, Waker)> = Vec::new();

pub fn early_init() {
    generic::init();
    TIMERS_INIT_STATE.store(1, Ordering::Relaxed);
}

pub fn init() {} // TODO: implement more timers

pub fn preferred_timer_ns() -> u64 {
    unsafe {
        let generic = crate::arch::drivers::time::generic::GENERIC_TIMER
            .get()
            .map(|t| t as &dyn KernelTimer);
        let timers: [&Option<&dyn KernelTimer>; 1] = [&generic];

        for timer in timers.into_iter().flatten() {
            if timer.is_supported() {
                return timer.elapsed_ns();
            }
        }

        0
    }
}

pub fn preferred_timer_ms() -> u64 {
    preferred_timer_ns() / 1_000_000
}

pub fn preferred_timer_pretty(digits: u32) -> String {
    crate::utils::time::elapsed_time_pretty(preferred_timer_ns(), digits)
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
