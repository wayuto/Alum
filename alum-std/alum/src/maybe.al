enum(pub) MaybeTag {
    Nothing,
    Just
}

struct(pub) Maybe<T> {
    tag: MaybeTag,
    value: T
}

fun(pub, pure) is_some<T>(m: Maybe<T>): bool {
    if m.tag == Just {
        return true
    }
    return false
}

