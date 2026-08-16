enum(pub) MaybeTag {
    Nothing,
    Just
}

struct(pub) Maybe<T> {
    tag: MaybeTag,
    value: T
}

fun(pub, pure) is_some<T>(m: Maybe<T>): int {
    if m.tag == Just {
        return 1
    }
    return 0
}

