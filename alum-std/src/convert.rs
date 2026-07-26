static mut BUF: [u8; 64] = [0; 64];

#[unsafe(no_mangle)]
pub extern "C" fn itoa(n: isize) -> *const u8 {
    unsafe {
        let buf = &raw mut BUF;

        if n == 0 {
            (*buf)[0] = b'0';
            (*buf)[1] = 0;
            return buf as *const u8;
        }

        let mut idx = 0;
        let mut num = n;
        let is_negative = num < 0;

        if is_negative {
            (*buf)[0] = b'-';
            idx = 1;
            num = -num;
        }

        let mut start = idx;
        let mut temp = num as usize;

        while temp > 0 {
            (*buf)[idx] = (temp % 10) as u8 + b'0';
            temp /= 10;
            idx += 1;
        }

        let mut end = idx - 1;
        while start < end {
            let tmp = (*buf)[start];
            (*buf)[start] = (*buf)[end];
            (*buf)[end] = tmp;
            start += 1;
            end -= 1;
        }

        (*buf)[idx] = 0;
        buf as *const u8
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

#[unsafe(no_mangle)]
pub extern "C" fn atof(s: *const u8) -> f64 {
    unsafe {
        if s.is_null() {
            return 0.0;
        }

        let mut ptr = s;
        let mut res = 0.0;
        let mut sign = 1.0;

        while *ptr == b' ' || *ptr == b'\t' || *ptr == b'\n' || *ptr == b'\r' {
            ptr = ptr.add(1);
        }

        if *ptr == b'-' {
            sign = -1.0;
            ptr = ptr.add(1);
        } else if *ptr == b'+' {
            ptr = ptr.add(1);
        }

        while *ptr >= b'0' && *ptr <= b'9' {
            res = res * 10.0 + (*ptr - b'0') as f64;
            ptr = ptr.add(1);
        }

        if *ptr == b'.' {
            ptr = ptr.add(1);
            let mut factor = 0.1;
            while *ptr >= b'0' && *ptr <= b'9' {
                res += (*ptr - b'0') as f64 * factor;
                factor /= 10.0;
                ptr = ptr.add(1);
            }
        }

        res * sign
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ftoa(n: f64) -> *const u8 {
    unsafe {
        let buf = &raw mut BUF;

        if n.is_nan() {
            (*buf)[0] = b'n';
            (*buf)[1] = b'a';
            (*buf)[2] = b'n';
            (*buf)[3] = 0;
            return buf as *const u8;
        }

        let mut num = n;
        let mut idx = 0;

        if num < 0.0 {
            (*buf)[idx] = b'-';
            idx += 1;
            num = -num;
        }

        let int_part_u64 = num as u64;
        let mut int_part = int_part_u64;
        let mut frac_part = num - (int_part_u64 as f64);

        let int_start = idx;
        if int_part == 0 {
            (*buf)[idx] = b'0';
            idx += 1;
        } else {
            while int_part > 0 {
                (*buf)[idx] = (int_part % 10) as u8 + b'0';
                int_part /= 10;
                idx += 1;
            }
            let mut s = int_start;
            let mut e = idx - 1;
            while s < e {
                let tmp = (*buf)[s];
                (*buf)[s] = (*buf)[e];
                (*buf)[e] = tmp;
                s += 1;
                e -= 1;
            }
        }

        (*buf)[idx] = b'.';
        idx += 1;

        for _ in 0..6 {
            frac_part *= 10.0;
            let digit = frac_part as u8;
            (*buf)[idx] = digit + b'0';
            idx += 1;
            frac_part -= digit as f64;
        }

        (*buf)[idx] = 0;
        buf as *const u8
    }
}
