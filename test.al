$import "io.al"

fun func(): void() {
    println("Called func()")
    return func    
}

fun main(): int {
    func()()
    return 0
}