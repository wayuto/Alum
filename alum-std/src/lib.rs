#![no_std]
#![no_main]
#![no_builtins]
#![feature(naked_functions)]

use core::arch::asm;
use core::panic::PanicInfo;

pub mod convert;
pub mod io;
pub mod memory;
pub mod string;

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}

unsafe extern "C" {
    fn main(argc: isize, argv: *const *const u8) -> isize;
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        asm!("ud2", options(noreturn));
    }
}

#[inline(always)]
pub extern "C" fn syscall(nr: usize, a1: isize, a2: isize, a3: isize) -> isize {
    let ret: isize;
    unsafe {
        asm!("
        mov rax, {nr} 
        mov rdi, {a1}
        mov rsi, {a2}
        mov rdx, {a3}
        syscall
        ", 
            nr = in(reg) nr as isize,
            a1 = in(reg) a1,
            a2 = in(reg) a2,
            a3 = in(reg) a3,
        lateout("rax") ret,
        clobber_abi("C"),
        );
    }
    ret
}

#[unsafe(no_mangle)]
pub extern "C" fn exit(code: isize) -> ! {
    unsafe {
        asm!("
            mov rax, 60    
            syscall
            ",
        in("rdi") code,
        options(noreturn, nostack)
        );
    }
    unreachable!()
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
extern "C" fn _start() -> ! {
    unsafe {
        core::arch::naked_asm!(
            "mov rdi, [rsp]",
            "lea rsi, [rsp + 8]",
            "and rsp, -16",
            "call _start_impl",
            "ud2",
        );
    }
}

#[unsafe(no_mangle)]
extern "C" fn _start_impl(argc: isize, argv: *const *const u8) -> ! {
    let mut arr = crate::memory::malloc(8 + argc as usize * 8) as *mut u8;
    unsafe {
        *(arr as *mut usize) = argc as usize;
        let mut data = arr.add(8) as *mut *const u8;
        for i in 0..argc {
            *data.add(i as usize) = *argv.add(i as usize);
        }
    }

    let ret = unsafe { main(argc, arr as *const *const u8) };
    exit(ret);
}
