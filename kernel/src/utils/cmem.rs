/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    arch::global_asm,
    ffi::{c_int, c_void},
};

global_asm!(
    "
memcpy:
    mov rcx, rdx
    mov rax, rdi
    rep movsb
    ret
.global memcpy

memset:
    push rdi
    mov rax, rsi
    mov rcx, rdx
    rep stosb
    pop rax
    ret
.global memset

memmove:
    mov rcx, rdx
    mov rax, rdi

    cmp rdi, rsi
    ja 1f

    rep movsb
    jmp 2f

1:
    lea rdi, [rdi + rcx - 1]
    lea rsi, [rsi + rcx - 1]
    std
    rep movsb
    cld

2:
    ret
.global memmove

memcmp:
    test rdx, rdx
    je 2f

    xor eax, eax
    nop word ptr [rax + rax]

1:
    movzx ecx, byte ptr [rsi + rax]
    cmp byte ptr [rdi + rax], cl
    jne 3f

    add rax, 1
    cmp rdx, rax
    jne 1b

2:
    xor eax, eax
    ret

3:
    setae al
    movzx eax, al
    add eax, eax
    add eax, -1
    ret
.global memcmp"
);

unsafe extern "C" {
    pub fn memcpy(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    pub fn memset(dest: *mut c_void, c: c_int, count: usize) -> *mut c_void;
    pub fn memmove(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, count: usize) -> c_int;
}
