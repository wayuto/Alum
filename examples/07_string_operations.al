import io
using io::{write, read, print, println}
import string
using string::{strlen, strcpy, memcmp}
import memory
using memory::malloc

// String Operations Example
// Demonstrates string manipulation
// NOTE: String literals are now stored in RW (read-write) section (as per modification)

fun main(): int {
    // String length
    var hello: string = "Hello, World!"
    var len: int = strlen(hello)
    print("Length of '")
    print(hello)
    println(f"': {len}")
    
    // String copy
    var copy: string = malloc(100)
    strcpy(copy, hello)
    
    println(f"Copy: {copy}")
    
    // String comparison using memcmp
    var result: int = memcmp(hello, copy, len + 1)
    if result == 0 {
        println("Strings are equal")
    } else {
        println("Strings are different")
    }
    
    // Different string
    var other: string = "Goodbye"
    var diff: int = memcmp(hello, other, strlen(other) + 1)
    if diff == 0 {
        println("Strings are equal")
    } else {
        println("Strings are different")
    }
    
    return 0
}



