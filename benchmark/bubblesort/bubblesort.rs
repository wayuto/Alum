fn sort(a: &mut [i32]) {
    let n = a.len();
    for j in 0..n - 1 {
        for k in 0..n - j - 1 {
            if a[k] > a[k + 1] {
                a.swap(k, k + 1);
            }
        }
    }
}

fn main() {
    let mut arr = [9, 2, 7, 1, 8, 3, 6, 4, 10, 5];
    sort(&mut arr);
    for x in arr {
        println!("{}", x);
    }
}