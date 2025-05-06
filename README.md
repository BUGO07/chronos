# ChronOS

A basic multi-arch hobby os held together by duct tape, written in rust.

Contributors are welcome.

:warning: This project is a mess, and will probably be so for a long time. 

## [LICENSE](LICENSE)

## Features

### x86_64
- Terminal Emulator
- Serial
- Interrupts
- PIT + TSC + KVM + HPET + LAPIC
- RTC
- Memory Management
- PS/2 Keyboard and Mouse
- Preemptive Scheduler (single core for now)
- ACPI
- Basic PCI
- Basic Shell

### aarch64 (arm64)

- Terminal Emulator
- Serial
- Generic Timer
- Memory Management (no pagemap yet)
- Cooperative Scheduler

## TODO:

- Preemptive Scheduler (aarch64)
- VFS
- USB
- Userspace
- Expand aarch64 support

## Known Bugs/Issues

- Address overflow in vmm on my laptop in debug builds.
- Crash on my laptop when setting the pagetable (this is fixed by hardsetting the pagesize to LARGE).
- No way to wake up from sleep yet.
- Using the keyboard before `up and running` makes keyboard and mouse not work

## Building And Running

Make sure you have the following installed:
* Rust
* Clang
* Make
* QEMU x86_64 | aarch64
* Xorriso

You will also need `aarch64-linux-gnu-gcc` to build for cross-compile c libraries

Follow these steps to build and run the os
1. Clone this repo with:\
``git clone --recursive --depth=1 https://github.com/BUGO07/chronos``

2. Go to the root directory of cloned repo and run:\
``make run`` For running debug mode\
``RUST_PROFILE="release" make run`` For running release mode\
``RUST_PROFILE="smol" make run`` For optimizing for size\
``make test`` For running tests

Change the KARCH environment variable to run `x86_64` (default) or `aarch64` (arm64) architectures (no tests for it yet)