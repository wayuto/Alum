$import "io.al"
$import "convert.al"

// User Input Example
// Demonstrates reading user input

fun main(): int {
    let name: string = malloc(100)
    let age_str: string = malloc(100)
    let age: int = 0
    
    // Get user's name
    println("Enter your name: ")
    name = input("")
    
    // Get user's age
    println("Enter your age: ")
    age_str = input("")
    age = atoi(age_str)
    
    // Display personalized message
    println("\nHello, ")
    println(name)
    print("You are ")
    print(itoa(age))
    println(" years old.")
    
    // Calculate age in 5 years
    let future_age: int = age + 5
    print("In 5 years, you will be ")
    print(itoa(future_age))
    println(" years old.")
    
    // Check if user is an adult
    if age >= 18 {
        println("You are an adult.")
    } else {
        println("You are a minor.")
    }
    
    return 0
}