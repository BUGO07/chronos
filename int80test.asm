BITS 64
SECTION .text
GLOBAL _start

_start:
.loop:
    mov rax, 0
    mov rdi, 420
    int 0x80
    jmp .loop
