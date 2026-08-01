$import "io.ah"

fun main(): void {
    let s = 0
    match s {
        0: {
            println("`s` is equal to 0")
        }
        1: {
            println("`s` is equal to 1")
        }
        default: {
            println("`s` is not equal to 0 or 1")
        }
    }
}