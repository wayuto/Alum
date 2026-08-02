$import "result.ah"
$import "io.ah"
$import "string.ah"
$import "convert.ah"

// Result Example

fun auth(password: string): Result<int, string> {
    if memcmp(password, "123456", 6) == 0 {
        return Result<int, string> {
            result: Ok, 
            value: ResultValue<int, string> {
                ok: 114514
            }
        }
    } else {
        return Result<int, string> {
            result: Err, 
            value: ResultValue<int, string> {
                err: "Wrong password!"
            }
        }
    }
}

fun main(): int {
    let password = input("Enter your password: ")
    let result = auth(password)
    match result.result {
        Ok: {
            println(f"{result.value.ok}")
        }
        Err: {
            println(f"{result.value.err}")
        }
    }
    return 0    
}