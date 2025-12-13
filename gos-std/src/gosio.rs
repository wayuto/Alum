use crate::{string::strlen, syscall};

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn write(fd: usize, buffer: *const u8, n: usize) -> isize {
    syscall(1, fd as isize, buffer as isize, n as isize)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn read(fd: usize, buffer: *mut u8, n: usize) -> isize {
    syscall(0, fd as isize, buffer as isize, n as isize)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn print(fmt: *const u8) -> isize {
    let len = strlen(fmt);
    write(1, fmt, len) + write(1, b"\n".as_ptr(), 1)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn mprint(args: *const *const u8) -> isize {
    if args.is_null() {
        return write(1, b"\n".as_ptr(), 1);
    }

    let mut ret: isize = 0;
    let mut i = 0;

    loop {
        unsafe {
            let current_string_ptr = *args.add(i);

            if current_string_ptr.is_null() {
                break;
            }

            let result = print(current_string_ptr);

            if result < 0 {
                return result;
            }
            ret += result;
            i += 1;
        }
    }
    ret
}

static mut BUFFER: [u8; 64] = [0; 64];

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn input(prompt: *const u8) -> *const u8 {
    let buffer = &raw mut BUFFER;

    if !prompt.is_null() {
        let mut prompt_len = 0;
        unsafe {
            while *prompt.add(prompt_len) != 0 {
                prompt_len += 1;
            }
        }

        if prompt_len > 0 {
            write(1, prompt, prompt_len);
        }
    }

    let mut total_read = 0;

    while total_read < unsafe { (*buffer).len() } - 1 {
        let mut ch: u8 = 0;

        let result = read(0, &mut ch as *mut u8, 1);

        if result <= 0 {
            break;
        }

        if ch == b'\n' || ch == b'\r' {
            break;
        }

        unsafe {
            (*buffer)[total_read] = ch;
        }
        total_read += 1;
    }
    unsafe {
        (*buffer)[total_read] = 0;
    }

    buffer as *const u8
}
