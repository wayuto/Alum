$import "io.ah"
$import "convert.ah"
$import "memory.ah"

// User Input Example
// Demonstrates reading user input

fun main(): int {
    var name: string = malloc(100)
    var age_str: string = malloc(100)
    var age: int = 0
    
    // Get user's name
    println("Enter your name: ")
    name = input("")
    
    // Get user's age
    println("Enter your age: ")
    age_str = input("")
    age = atoi(age_str)
    
    // Display personalized message
    println(f"Hello, {name}!")
    println(f"You are {age} years old.")
    
    // Calculate age in 5 years
    var future_age: int = age + 5
    println(f"In 5 years, you will be {future_age} years old.")
    
    // Check if user is an adult
    if age >= 18 {
        println("You are an adult.")
    } else {
        println("You are a minor.")
    }
    
    return 0
}