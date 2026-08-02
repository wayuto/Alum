$import "io.ah"
$import "convert.ah"

// Type Conversion Example
// Demonstrates type conversion functions

fun main(): int {
    // Integer to string
    let num: int = 42
    let str_num: string = itoa(num)
    
    println(f"Integer {num} as string: {str_num}")
    
    // Negative integer to string
    let neg_num: int = -123
    let str_neg: string = itoa(neg_num)
    
    println(f"Negative integer {neg_num} as string: {str_neg}")
    
    // String to integer
    let str: string = "100"
    let parsed_int: int = atoi(str)
    
    println(f"String \"{str}\" as integer: {parsed_int}")
    
    // Float to string
    let pi: float = 3.14159
    let str_float: string = ftoa(pi)
    
    println(f"Float {pi} as string: {str_float}")
    
    // String to float
    let float_str: string = "2.718"
    let parsed_float: float = atof(float_str)
    
    println(f"String \"{float_str}\" as float: {parsed_float}")
    
    // Arithmetic on converted values
    let sum: int = parsed_int + parsed_int
    println(f"Sum of parsed integers: {sum}")
    
    return 0
}