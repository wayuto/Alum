static mut BUFFER: [u8; 64] = [0; 64];

#[unsafe(no_mangle)]
pub extern "C" fn itoa(n: isize) -> *const u8 {
    unsafe {
        let buffer = &raw mut BUFFER;

        if n == 0 {
            (*buffer)[0] = b'0';
            (*buffer)[1] = 0;
            return buffer as *const u8;
        }

        let mut idx = 0;
        let mut num = n;
        let is_negative = num < 0;

        if is_negative {
            (*buffer)[0] = b'-';
            idx = 1;
            num = -num;
        }

        let mut start = idx;
        let mut temp = num as usize;

        while temp > 0 {
            (*buffer)[idx] = (temp % 10) as u8 + b'0';
            temp /= 10;
            idx += 1;
        }

        let mut end = idx - 1;
        while start < end {
            let tmp = (*buffer)[start];
            (*buffer)[start] = (*buffer)[end];
            (*buffer)[end] = tmp;
            start += 1;
            end -= 1;
        }

        (*buffer)[idx] = 0;
        buffer as *const u8
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn atoi(s: *const u8) -> isize {
    unsafe {
        if s.is_null() {
            return 0;
        }

        let mut ptr = s;
        let mut result: isize = 0;
        let mut sign: isize = 1;
        let mut is_neg = false;

        while *ptr == b' ' || *ptr == b'\t' || *ptr == b'\n' || *ptr == b'\r' {
            ptr = ptr.add(1);
        }

        match *ptr {
            b'+' => {
                sign = 1;
                ptr = ptr.add(1);
                is_neg = true;
            }
            b'-' => {
                sign = -1;
                ptr = ptr.add(1);
                is_neg = true;
            }
            _ => {}
        }

        if is_neg {
            while *ptr == b' ' || *ptr == b'\t' || *ptr == b'\n' || *ptr == b'\r' {
                ptr = ptr.add(1);
            }
        }

        while *ptr >= b'0' && *ptr <= b'9' {
            let digit = (*ptr - b'0') as isize;

            result = result * 10 + digit;
            ptr = ptr.add(1);
        }

        result * sign
    }
}
