const fn factorial(n: u64) -> u64 {
    if n < 2 {
        return 1;
    }
    n * factorial(n - 1)
}

fn main() {
    const RESULT: u64 = factorial(20);
    println!("{}", RESULT);
}