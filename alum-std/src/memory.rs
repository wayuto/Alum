use crate::syscall;

#[unsafe(no_mangle)]
pub extern "C" fn malloc(size: usize) -> *mut u8 {
    let old_brk = syscall(12, 0, 0, 0);
    let new_brk = old_brk + size as isize;
    syscall(12, new_brk, 0, 0);
    old_brk as *mut u8
}
