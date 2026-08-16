enum ResultStatus {
    Ok,
    Err
}

union ResultValue<T, E> {
    ok: T, 
    err: E
}

struct Result<T, E> {
    result: ResultStatus, 
    value: ResultValue<T, E>
}

