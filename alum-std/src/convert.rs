static mut BUF: [u8; 64] = [0; 64];

#[unsafe(no_mangle)]
pub extern "C" fn atof(s: *const u8) -> f64 {
    unsafe {
        if s.is_null() {
            return 0.0;
        }

        let mut ptr = s;
        let mut res = 0.0;
        let mut sign = 1.0;
        let mut any_digit = false;

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
            any_digit = true;
            ptr = ptr.add(1);
        }

        if *ptr == b'.' {
            ptr = ptr.add(1);
            let mut factor = 0.1;
            while *ptr >= b'0' && *ptr <= b'9' {
                res += (*ptr - b'0') as f64 * factor;
                factor /= 10.0;
                any_digit = true;
                ptr = ptr.add(1);
            }
        }

        res * if any_digit { sign } else { 1.0 }
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

        if n.is_infinite() {
            let mut idx = 0;
            if n.is_sign_negative() {
                (*buf)[idx] = b'-';
                idx += 1;
            }
            (*buf)[idx] = b'i';
            (*buf)[idx + 1] = b'n';
            (*buf)[idx + 2] = b'f';
            (*buf)[idx + 3] = 0;
            return buf as *const u8;
        }

        let mut num = n;
        let mut idx = 0;

        if num.is_sign_negative() {
            (*buf)[idx] = b'-';
            idx += 1;
            num = -num;
        }

        let mut frac_part: f64;

        let int_start = idx;
        if num >= 1e18 {
            let truncated = num as i128;
            let mut digits = [0u8; 48];
            let mut cnt = 0usize;
            let mut v = truncated;
            if v == 0 {
                cnt = 1;
            }
            while v > 0 && cnt < digits.len() {
                digits[cnt] = (v % 10) as u8;
                v /= 10;
                cnt += 1;
            }
            for j in (0..cnt).rev() {
                (*buf)[idx] = digits[j] + b'0';
                idx += 1;
            }
            frac_part = num - truncated as f64;
        } else {
            let int_part_u64 = num as u64;
            let mut int_part = int_part_u64;
            frac_part = num - (int_part_u64 as f64);

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
        }

        (*buf)[idx] = b'.';
        idx += 1;

        for _ in 0..7 {
            frac_part *= 10.0;
            let digit = frac_part as u8;
            (*buf)[idx] = digit + b'0';
            idx += 1;
            frac_part -= digit as f64;
        }

        let round_up = (*buf)[idx - 1] >= b'5';
        idx -= 1;
        if round_up {
            let mut p = idx;
            loop {
                p -= 1;
                let c = (*buf)[p];
                if c == b'.' {
                    continue;
                }
                if c == b'9' {
                    (*buf)[p] = b'0';
                    if p == int_start {
                        let mut q = idx;
                        while q > int_start {
                            (*buf)[q] = (*buf)[q - 1];
                            q -= 1;
                        }
                        (*buf)[int_start] = b'1';
                        idx += 1;
                        break;
                    }
                    continue;
                }
                (*buf)[p] += 1;
                break;
            }
        }

        (*buf)[idx] = 0;
        buf as *const u8
    }
}
