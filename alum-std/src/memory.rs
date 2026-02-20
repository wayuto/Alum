use crate::syscall;

#[unsafe(no_mangle)]
pub extern "C" fn malloc(size: usize) -> *mut u8 {
    let old_brk = syscall(12, 0, 0, 0);
    let new_brk = old_brk + size as isize;
    syscall(12, new_brk, 0, 0);
    old_brk as *mut u8
}

#[unsafe(no_mangle)]
pub extern "C" fn free(ptr: *mut u8, size: usize) {
    if ptr.is_null() || size == 0 {
        return;
    }
    let current_brk = syscall(12, 0, 0, 0);
    let ptr_addr = ptr as isize;
    let ptr_end = ptr_addr + size as isize;
    if ptr_end == current_brk {
        syscall(12, ptr_addr, 0, 0);
    }
}
