#[unsafe(no_mangle)]
pub extern "C" fn strlen(ptr: *const u8) -> usize {
    let mut len = 0;
    let mut p = ptr;
    unsafe {
        while *p != b'\0' {
            len += 1;
            p = p.add(1);
        }
    }
    len
}

#[unsafe(no_mangle)]
pub extern "C" fn strcpy(dst: *mut u8, src: *const u8) -> *mut u8 {
    unsafe {
        let mut i = 0usize;
        loop {
            let b = *src.add(i);
            *dst.add(i) = b;
            if b == 0 {
                break;
            }
            i += 1;
        }
    }
    dst
}

#[unsafe(no_mangle)]
pub extern "C" fn strcmp(a: *const u8, b: *const u8) -> i32 {
    unsafe {
        let mut i = 0usize;
        loop {
            let ca = *a.add(i);
            let cb = *b.add(i);
            if ca != cb {
                return ca as i32 - cb as i32;
            }
            if ca == 0 {
                return 0;
            }
            i += 1;
        }
    }
}
