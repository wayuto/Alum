use crate::syscall;
use core::ptr;

const ALIGN: usize = 8;
const HEADER_SIZE: usize = 8;
const FOOTER_SIZE: usize = 8;
const MIN_BLOCK_SIZE: usize = 32;

static mut FREE_LIST: *mut u8 = ptr::null_mut();
static mut HEAP_START: *mut u8 = ptr::null_mut();
static mut HEAP_END: *mut u8 = ptr::null_mut();

fn brk(addr: usize) -> usize {
    syscall(12, addr as isize, 0, 0) as usize
}

fn align_up(n: usize) -> usize {
    (n + ALIGN - 1) & !(ALIGN - 1)
}

fn block_size(b: *mut u8) -> usize {
    unsafe { (*(b as *const usize)) & !1 }
}

fn is_free(b: *mut u8) -> bool {
    unsafe { (*(b as *const usize)) & 1 == 1 }
}

fn set_size(b: *mut u8, size: usize, free: bool) {
    unsafe { *(b as *mut usize) = size | (free as usize) };
}

fn set_footer(b: *mut u8, size: usize, free: bool) {
    unsafe { *(b.add(size).sub(FOOTER_SIZE) as *mut usize) = size | (free as usize) };
}

fn next_free(b: *mut u8) -> *mut u8 {
    unsafe { *(b.add(HEADER_SIZE) as *const *mut u8) }
}

fn set_next_free(b: *mut u8, n: *mut u8) {
    unsafe { *(b.add(HEADER_SIZE) as *mut *mut u8) = n };
}

fn insert_free(b: *mut u8) {
    unsafe {
        if FREE_LIST.is_null() || b < FREE_LIST {
            set_next_free(b, FREE_LIST);
            FREE_LIST = b;
            return;
        }
        let mut cur = FREE_LIST;
        loop {
            let n = next_free(cur);
            if n.is_null() || b < n {
                set_next_free(b, n);
                set_next_free(cur, b);
                return;
            }
            cur = n;
        }
    }
}

fn remove_free(b: *mut u8) {
    unsafe {
        let mut prev: *mut u8 = ptr::null_mut();
        let mut cur = FREE_LIST;
        while !cur.is_null() && cur != b {
            prev = cur;
            cur = next_free(cur);
        }
        if cur.is_null() {
            return;
        }
        if prev.is_null() {
            FREE_LIST = next_free(cur);
        } else {
            set_next_free(prev, next_free(cur));
        }
    }
}

fn grow_heap(need: usize) -> *mut u8 {
    unsafe {
        let cur = brk(0);
        let end = brk(cur + need);
        if end < cur + need {
            return ptr::null_mut();
        }
        let b = cur as *mut u8;
        if HEAP_START.is_null() {
            HEAP_START = b;
        }
        HEAP_END = end as *mut u8;
        let size = end - cur;
        set_size(b, size, true);
        set_footer(b, size, true);
        b
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn malloc(size: usize) -> *mut u8 {
    let size = if size == 0 { 1 } else { size };
    let mut need = align_up(size) + HEADER_SIZE + FOOTER_SIZE;
    if need < MIN_BLOCK_SIZE {
        need = MIN_BLOCK_SIZE;
    }

    unsafe {
        let mut prev: *mut u8 = ptr::null_mut();
        let mut cur = FREE_LIST;
        while !cur.is_null() {
            let sz = block_size(cur);
            if sz >= need {
                if prev.is_null() {
                    FREE_LIST = next_free(cur);
                } else {
                    set_next_free(prev, next_free(cur));
                }
                if sz >= need + MIN_BLOCK_SIZE {
                    let rem = cur.add(need);
                    set_size(rem, sz - need, true);
                    set_footer(rem, sz - need, true);
                    insert_free(rem);
                    set_size(cur, need, false);
                    set_footer(cur, need, false);
                } else {
                    set_size(cur, sz, false);
                    set_footer(cur, sz, false);
                }
                return cur.add(HEADER_SIZE);
            }
            prev = cur;
            cur = next_free(cur);
        }

        let b = grow_heap(need);
        if b.is_null() {
            return ptr::null_mut();
        }
        let sz = block_size(b);
        if sz > need {
            let rem = b.add(need);
            set_size(rem, sz - need, true);
            set_footer(rem, sz - need, true);
            insert_free(rem);
        }
        set_size(b, need, false);
        set_footer(b, need, false);
        b.add(HEADER_SIZE)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let mut b = ptr.sub(HEADER_SIZE);
        if is_free(b) {
            return;
        }
        let mut sz = block_size(b);

        let nb = b.add(sz);
        if nb < HEAP_END && is_free(nb) {
            remove_free(nb);
            sz += block_size(nb);
        }
        set_size(b, sz, true);
        set_footer(b, sz, true);

        if b > HEAP_START {
            let prev_footer = *(b.sub(FOOTER_SIZE) as *const usize);
            let prev_sz = prev_footer & !1;
            let pb = b.sub(prev_sz);
            if pb >= HEAP_START && is_free(pb) {
                remove_free(pb);
                let new_sz = block_size(pb) + sz;
                set_size(pb, new_sz, true);
                set_footer(pb, new_sz, true);
                b = pb;
            }
        }
        insert_free(b);
    }
}
