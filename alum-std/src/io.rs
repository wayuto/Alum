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
    write(1, fmt, len)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn println(fmt: *const u8) -> isize {
    let len = strlen(fmt);
    write(1, fmt, len) + write(1, b"\n".as_ptr(), 1)
}

static mut BUFFER: [u8; 1024] = [0; 1024];

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

#[unsafe(no_mangle)]
pub extern "C" fn fopen(filename: *const u8, flags: isize, mode: isize) -> isize {
    syscall(2, filename as isize, flags, mode)
}

#[unsafe(no_mangle)]
pub extern "C" fn fclose(fd: isize) -> isize {
    syscall(3, fd, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn fread(fd: isize) -> *const u8 {
    let buffer = &raw mut BUFFER;

    syscall(0, fd, buffer as isize, 1024);
    buffer as *const u8
}

#[unsafe(no_mangle)]
pub extern "C" fn fwrite(fd: isize, buf: *const u8, n: usize) -> isize {
    syscall(1, fd, buf as isize, n as isize)
}

#[unsafe(no_mangle)]
pub extern "C" fn lseek(fd: isize, off: isize, whence: isize) -> isize {
    syscall(8, fd, off, whence)
}
