const fn fib(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    let mut a = 0u64;
    let mut b = 1u64;
    let mut i = 0;
    while i < n - 1 {
        let next = a + b;
        a = b;
        b = next;
        i += 1;
    }
    b
}

fn main() {
    const RESULT: u64 = fib(40);
    println!("{}", RESULT);
}