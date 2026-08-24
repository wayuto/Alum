#![no_std]
#![no_main]
#![no_builtins]

use core::arch::asm;
use core::panic::PanicInfo;

pub mod convert;
pub mod io;
pub mod memory;
pub mod string;
pub mod sys;

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

#[unsafe(no_mangle)]
#[unsafe(naked)]
extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        "mov rdi, [rsp]",
        "lea rsi, [rsp + 8]",
        "and rsp, -16",
        "call _start_impl",
        "ud2",
    );
}

#[unsafe(no_mangle)]
extern "C" fn _start_impl(argc: isize, argv: *const *const u8) -> ! {
    let arr = unsafe { crate::memory::malloc(8 + argc as usize * 8) } as *mut u8;
    unsafe {
        *(arr as *mut usize) = argc as usize;
        let data = arr.add(8) as *mut *const u8;
        for i in 0..argc {
            *data.add(i as usize) = *argv.add(i as usize);
        }
    }

    let ret = unsafe { main(argc, arr as *const *const u8) };
    crate::sys::exit(ret);
}
