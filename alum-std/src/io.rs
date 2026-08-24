use crate::{string::strlen, sys};

const BUFFER_SIZE: usize = 1024;

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn print(fmt: *const u8) -> isize {
    let len = unsafe { strlen(fmt as *const i8) };
    sys::write(1, fmt, len)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn println(fmt: *const u8) -> isize {
    let len = unsafe { strlen(fmt as *const i8) };
    sys::write(1, fmt, len) + sys::write(1, b"\n".as_ptr(), 1)
}

static mut BUF: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn input(prompt: *const u8) -> *const u8 {
    let buf = &raw mut BUF;

    if !prompt.is_null() {
        let mut prompt_len = 0;
        unsafe {
            while *prompt.add(prompt_len) != 0 {
                prompt_len += 1;
            }
        }

        if prompt_len > 0 {
            sys::write(1, prompt, prompt_len);
        }
    }

    let mut total_read = 0;

    while total_read < unsafe { (*buf).len() } - 1 {
        let mut ch: u8 = 0;

        let result = sys::read(0, &mut ch as *mut u8, 1);

        if result <= 0 {
            break;
        }

        if ch == b'\n' {
            break;
        }

        if ch == b'\r' {
            let mut nxt: u8 = 0;
            let r = sys::read(0, &mut nxt as *mut u8, 1);
            if r > 0 && nxt == b'\n' {}
            break;
        }

        unsafe {
            (*buf)[total_read] = ch;
        }
        total_read += 1;
    }
    unsafe {
        (*buf)[total_read] = 0;
    }

    let out = unsafe { crate::memory::malloc(total_read + 1) } as *mut u8;
    unsafe {
        core::ptr::copy_nonoverlapping(buf.cast::<u8>(), out, total_read + 1);
    }
    out as *const u8
}

#[unsafe(no_mangle)]
pub extern "C" fn fopen(filename: *const u8, flags: isize, mode: isize) -> isize {
    sys::open(filename, flags, mode)
}

#[unsafe(no_mangle)]
pub extern "C" fn fclose(fd: isize) -> isize {
    sys::close(fd)
}

static mut BUF_READ: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

#[unsafe(no_mangle)]
pub extern "C" fn fread(fd: isize) -> *const u8 {
    let buf = &raw mut BUF_READ;

    let mut total = 0usize;
    unsafe {
        while total < BUFFER_SIZE - 1 {
            let n = sys::read(fd, (*buf).as_mut_ptr().add(total), BUFFER_SIZE - 1 - total);
            let n = if n < 0 { 0isize } else { n };
            if n == 0 {
                break;
            }
            total += n as usize;
        }
        (*buf)[total] = 0;
    }
    buf as *const u8
}

#[unsafe(no_mangle)]
pub extern "C" fn fwrite(fd: isize, buf: *const u8, n: usize) -> isize {
    sys::write(fd, buf, n)
}
