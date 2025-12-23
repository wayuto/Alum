section .data
.S.0 db 1.5, 0
section .text
global main
main:
push rbp
mov rbp, rsp
sub rsp, 32
.L_main_loop:
movsd rax, [.S.0]
mov [rbp - 8], rax
mov [rbp - 16], rax
mov rax, [rbp - 24]
jmp .L_main_exit
.L_main_exit:
leave
ret
