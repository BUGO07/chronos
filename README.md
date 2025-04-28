# ChronOS

A basic x86_64 hobby os made in rust.

Contributors are welcome.

:warning: This project is a mess, and will probably be so for a long time. 

## [LICENSE](LICENSE)

## Features

- Basic Logger
- Serial
- Interrupts
- PIT + TSC + KVM + HPET + LAPIC
- RTC
- Memory Management
- PS/2 Keyboard
- PS/2 Mouse
- Cooperative Scheduler
- ACPI
- Basic Shell (i know its not supposed to be there, its temporary)

## TODO:

- Preemptive Scheduler
- PCI
- VFS
- USB
- Userspace

## Known Bugs/Issues

- Sometimes address overflow in vmm on real hardware in debug builds.
- Reboot causes a pagefault after booting.
- No way to wake up from sleep yet.
- Opt-level 2 and 3 cause a bootloop

## Building And Running

Make sure you have the following installed:
* Rust
* Clang
* Make
* QEMU x86-64
* Xorriso

Follow these steps to build and run the os
1. Clone this repo with:\
``git clone --recursive --depth=1 https://github.com/BUGO07/chronos``

2. Go to the root directory of cloned repo and run:\
``make run`` For running debug mode\
``RUST_PROFILE="release" make run`` For running release mode\
``RUST_PROFILE="smol" make run`` For optimizing for size\
``make test`` For running tests