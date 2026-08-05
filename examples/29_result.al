$import "result.ah"
$import "io.ah"

// Result Example

fun auth(password: string): Result<int, string> {
    if password == "123456" {
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
    var password = input("Enter your password: ")
    var result = auth(password)
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