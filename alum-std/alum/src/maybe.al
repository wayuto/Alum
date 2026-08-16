enum MaybeTag {
    Nothing,
    Just
}

struct Maybe<T> {
    tag: MaybeTag,
    value: T
}

fun(pure) is_some<T>(m: Maybe<T>): int {
    if m.tag == Just {
        return 1
    }
    return 0
}

