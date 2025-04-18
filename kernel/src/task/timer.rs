/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll, Waker},
};

use alloc::{format, string::String, vec::Vec};
use spin::Mutex;

static TICKS: AtomicU64 = AtomicU64::new(0);

pub fn tick_timer() {
    TICKS.fetch_add(1, Ordering::Relaxed);
    wake_ready_tasks();
}

pub fn current_ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

pub fn pit_time_pretty(digits: u32) -> String {
    let elapsed_ns = current_ticks() * 1_000_000;
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
            wake_at: current_ticks() + duration_ticks,
        }
    }
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if current_ticks() >= self.wake_at {
            Poll::Ready(())
        } else {
            register_sleep(self.wake_at, cx.waker().clone());
            Poll::Pending
        }
    }
}

lazy_static::lazy_static! {
    static ref SLEEPING_TASKS: Mutex<Vec<(u64, Waker)>> = Mutex::new(Vec::new());
}

pub fn register_sleep(wake_at: u64, waker: Waker) {
    SLEEPING_TASKS.lock().push((wake_at, waker));
}

pub fn wake_ready_tasks() {
    let now = current_ticks();
    let mut tasks = SLEEPING_TASKS.lock();
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
