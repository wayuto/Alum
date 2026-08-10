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
pub unsafe extern "C" fn strcmp(s1: *const u8, s2: *const u8) -> i32 {
    let mut i = 0;
    loop {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return (a as i32) - (b as i32);
        }
        if a == b'\0' {
            return 0;
        }
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        unsafe {
            let a = *s1.add(i);
            let b = *s2.add(i);
            if a != b {
                return (a as i32) - (b as i32);
            }
        }
        i += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    unsafe { bcmp(s1, s2, n) }
}
