section .bss
buf:resb 32

section .text
global itoa
global atoi

itoa:
mov rcx, buf+31
xor eax, eax
mov byte [rcx], 0
dec rcx
mov rdx, rdi
xor r8, r8
cmp rdi, 0
jge .positive
mov r8b, '-'
neg rdx
.positive:
mov rax, rdx
.digit_loop:
xor edx, edx
mov rbx, 10
div rbx
add dl, '0'
mov[rcx], dl
dec rcx
test rax, rax
jnz .digit_loop
test r8b, r8b
jz .no_sign
mov[rcx], r8b
dec rcx
.no_sign:
lea rax, [rcx+1]
ret 

atoi:
xor eax, eax 
xor ecx, ecx 
mov rsi, rdi 
.skip_ws:
movzx edx, byte [rsi]
inc rsi
cmp dl, ' '
je .skip_ws
dec rsi 
movzx edx, byte [rsi]
cmp dl, '-'
jne .check_plus
mov ecx, -1
inc rsi
jmp .convert
.check_plus:
cmp dl, '+'
jne .convert
inc rsi
.convert:
xor rax, rax 
mov r8, 10
.next_digit:
movzx edx, byte [rsi]
sub dl, '0'
cmp dl, 9
ja .done 
imul r8 
movsxd rdx, edx
add rax, rdx 
inc rsi
jmp .next_digit
.done:
test ecx, ecx
jns .finish
neg rax 
.finish:
ret