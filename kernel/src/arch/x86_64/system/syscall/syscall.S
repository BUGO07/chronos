
.extern syscall_handler
syscall_entry:
    swapgs

    mov gs:[8], rsp
    mov rsp, gs:[16]

    push 0x23 # ss
    push gs:[8] # rsp
    push r11 # rflags
    push 0x2B # cs
    push rcx # rip
    push 0 # error_code
    push 0 # vector

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

    sti
    mov rdi, rsp
    call syscall_handler
    cli

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

    pop rcx
    add rsp, 8
    pop r11
    pop rsp

    swapgs
    sysretq
.global syscall_entry