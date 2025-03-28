# Rust Kernel V2

A basic x86_64 os kernel made in rust (rewrite of rusty_os)\
Contributors are welcome

## [LICENSE](LICENSE)

## Features

- Basic Logger
- Serial

## Building And Running

Make sure you have the following installed:
* Rust
* Make
* QEMU x86-64
* Xorriso

Follow these steps to build and run the os
1. Clone this repo with:\
``git clone https://github.com/BUGO07/kernel``

2. Go to the root directory of cloned repo and run:\
``make run`` For UEFI mode\
``make run-bios`` For BIOS mode\
``make test`` For running tests