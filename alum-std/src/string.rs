pub(crate) fn strlen(ptr: *const u8) -> usize {
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
