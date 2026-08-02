$import "io.ah"

// Variables Example
// Demonstrates variable declarations with different types

fun main(): int {
    // Integer variable
    let age: int = 25
    
    // Float variable
    let pi: float = 3.14159
    
    // Boolean variable
    let is_student: bool = true
    
    // String variable
    let name: string = "Alum"
    
    // Nil value for integer
    let empty: int = nil
    
    // Print values
    println(f"Name: {name}")
    println(f"Age: {age}")
    println(f"Pi: {pi}")
    println(f"Is Student: {if is_student { "true" } else { "false" }}")
    
    return 0
}