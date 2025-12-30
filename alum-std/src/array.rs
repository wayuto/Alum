const MAX_RANGE_LEN: usize = 1024;
const POOL_SIZE: usize = 16;
static mut RANGE_POOL: [[isize; MAX_RANGE_LEN + 1]; POOL_SIZE] =
    [[0; MAX_RANGE_LEN + 1]; POOL_SIZE];
static mut POOL_IDX: usize = 0;

#[unsafe(no_mangle)]
pub extern "C" fn range(start: isize, end: isize) -> *mut isize {
    unsafe {
        let len = if end <= start {
            0
        } else {
            (end - start) as usize
        };
        let final_len = if len > MAX_RANGE_LEN {
            MAX_RANGE_LEN
        } else {
            len
        };
        let buf_idx = POOL_IDX % POOL_SIZE;
        POOL_IDX += 1;
        let base_ptr = &mut RANGE_POOL[buf_idx][0] as *mut isize;
        *(base_ptr as *mut usize) = final_len;
        let data_ptr = base_ptr.add(1);
        for i in 0..final_len {
            *data_ptr.add(i) = start + i as isize;
        }

        base_ptr
    }
}
