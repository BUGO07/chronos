.extern isr_handler
isr_common_stub:
    cld

    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    mov rdi, rsp
    call isr_handler

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    add rsp, 16

    iretq

.macro isr number
    isr_\number:
.if !(\number == 8 || (\number >= 10 && \number <= 14) || \number == 17 || \number == 21 || \number == 29 || \number == 30)
    push 0
.endif
    push \number
    jmp isr_common_stub
.endm

.altmacro
.macro isr_insert number
    .section .text
    isr \number

    .section .data
    .quad isr_\number
.endm

.section .data
.byte 1
.align 8
isr_table:
.set i, 0
.rept 256
    isr_insert %i
    .set i, i + 1
.endr
.global isr_table