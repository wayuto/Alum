$import "io.al"
$import "convert.al"

// Type Conversion Example
// Demonstrates type conversion functions

fun main(): int {
    // Integer to string
    let num: int = 42
    let str_num: string = itoa(num)
    
    print("Integer ")
    print(itoa(num))
    print(" as string: ")
    println(str_num)
    
    // Negative integer to string
    let neg_num: int = -123
    let str_neg: string = itoa(neg_num)
    
    print("Negative integer ")
    print(itoa(neg_num))
    print(" as string: ")
    println(str_neg)
    
    // String to integer
    let str: string = "100"
    let parsed_int: int = atoi(str)
    
    print("String \"")
    print(str)
    print("\" as integer: ")
    println(itoa(parsed_int))
    
    // Float to string
    let pi: float = 3.14159
    let str_float: string = ftoa(pi)
    
    print("Float ")
    print(ftoa(pi))
    print(" as string: ")
    println(str_float)
    
    // String to float
    let float_str: string = "2.718"
    let parsed_float: float = atof(float_str)
    
    print("String \"")
    print(float_str)
    print("\" as float: ")
    println(ftoa(parsed_float))
    
    // Arithmetic on converted values
    let sum: int = parsed_int + parsed_int
    print("Sum of parsed integers: ")
    println(itoa(sum))
    
    return 0
}