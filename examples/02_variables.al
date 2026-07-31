$import "io.ah"
$import "convert.ah"

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
    println("Name: ")
    println(name)
    println("Age: ")
    println(itoa(age))
    println("Pi: ")
    println(ftoa(pi))
    println("Is Student: ")
    if is_student {
        println("true")
    } else {
        println("false")
    }
    
    return 0
}