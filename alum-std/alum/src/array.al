import maybe
using maybe::{Maybe}

struct(pub) Array<T> {
    data: T[],
    len: int,
    iter: int,
    nth: T(*Array<T>, int),
    set_nth: void(*Array<T>, int, T),
    next: Maybe<T>(*Array<T>),
}

fun(pub) arr_new<T>(data: T[], len: int): Array<T> {
    return Array<T> {
        data: data,
        len: len,
        iter: 0,
        nth: \(v: *Array<T>, i: int): T {
            if i < 0 || i >= v.len return nil
            return v.data[i]
        },
        set_nth: \(v: *Array<T>, i: int, elem: T): void {
            if i < 0 || i >= v.len return
            v.data[i] = elem
        },
        next: \(v: *Array<T>): Maybe<T> {
            if v.iter >= v.len {
                v.iter = 0
                return Maybe<T> {
                    tag: Nothing,
                    value: nil
                }
            }
            var elem: T = v.data[v.iter]
            v.iter = v.iter + 1
            return Maybe<T> {
                tag: Just,
                value: elem
            }
        },
    }
}

