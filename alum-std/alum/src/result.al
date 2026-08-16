enum(pub) ResultStatus {
    Ok,
    Err
}

union(pub) ResultValue<T, E> {
    ok: T, 
    err: E
}

struct(pub) Result<T, E> {
    result: ResultStatus, 
    value: ResultValue<T, E>
}

