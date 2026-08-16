using io::println

// Variables Example
// Demonstrates variable declarations with different types

fun main(): int {
    // Integer variable
    var age: int = 25
    
    // Float variable
    var pi: float = 3.14159
    
    // Boolean variable
    var is_student: bool = true
    
    // String variable
    var name: string = "Alum"
    
    // Nil value for integer
    var empty: int = nil
    
    // Print values
    println(f"Name: {name}")
    println(f"Age: {age}")
    println(f"Pi: {pi}")
    println(f"Is Student: {if is_student { "true" } else { "false" }}")
    
    return 0
}


