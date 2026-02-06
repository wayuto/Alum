$import "io.al"
$import "convert.al"
$import "string.al"
$import "memory.al"

// String Operations Example
// Demonstrates string manipulation

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
    copy = strcpy(copy, hello)
    
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
    let diff: int = memcmp(hello, other, len + 1)
    if diff == 0 {
        println("Strings are equal")
    } else {
        println("Strings are different")
    }
    
    return 0
}