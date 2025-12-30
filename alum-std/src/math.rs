#[unsafe(no_mangle)]
pub extern "C" fn abs(x: isize) -> isize {
    if x < 0 { -x } else { x }
}

#[unsafe(no_mangle)]
pub extern "C" fn sqrt(x: isize) -> isize {
    x * x
}

#[unsafe(no_mangle)]
pub extern "C" fn max(a: isize, b: isize) -> isize {
    if a > b { a } else { b }
}

#[unsafe(no_mangle)]
pub extern "C" fn min(a: isize, b: isize) -> isize {
    if a < b { a } else { b }
}

#[unsafe(no_mangle)]
pub extern "C" fn pow(base: isize, exp: isize) -> isize {
    if exp == 0 {
        return 1;
    }
    let mut res: isize = 1;
    for _ in 0..exp {
        res *= base;
    }
    res
}

#[unsafe(no_mangle)]
pub extern "C" fn fact(n: isize) -> isize {
    if n <= 1 {
        return 1;
    }

    let mut result: isize = 1;
    for i in 2..=n {
        result = result * i;
    }

    result
}
