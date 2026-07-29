$import "io.ah"
$import "convert.ah"
$import "string.ah"
$import "memory.ah"

// String Operations Example
// Demonstrates string manipulation
// NOTE: String literals are now stored in RW (read-write) section (as per modification)

fun main(): int {
    // String length
    let hello: string = "Hello, World!"
    let len: int = strlen(hello)
    print("Length of '")
    print(hello)
    print("': ")
    println(itoa(len))
    
    // String copy
    let copy: string = malloc(100)
    strcpy(copy, hello)
    
    print("Copy: ")
    println(copy)
    
    // String comparison using memcmp
    let result: int = memcmp(hello, copy, len + 1)
    if result == 0 {
        println("Strings are equal")
    } else {
        println("Strings are different")
    }
    
    // Different string
    let other: string = "Goodbye"
    let diff: int = memcmp(hello, other, strlen(other) + 1)
    if diff == 0 {
        println("Strings are equal")
    } else {
        println("Strings are different")
    }
    
    return 0
}