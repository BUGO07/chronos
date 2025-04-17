/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll, Waker},
};

use alloc::vec::Vec;
use spin::Mutex;

static TICKS: AtomicU64 = AtomicU64::new(0);

pub fn tick_timer() {
    TICKS.fetch_add(1, Ordering::Relaxed);
    wake_ready_tasks();
}

pub fn current_ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
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
