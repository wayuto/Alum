import io
using io::println
import convert
using convert::{itoa, atoi, atof, ftoa}

// Type Conversion Example
// Demonstrates type conversion functions

fun main(): int {
    // Integer to string
    var num: int = 42
    var str_num: string = itoa(num)
    
    println(f"Integer {num} as string: {str_num}")
    
    // Negative integer to string
    var neg_num: int = -123
    var str_neg: string = itoa(neg_num)
    
    println(f"Negative integer {neg_num} as string: {str_neg}")
    
    // String to integer
    // NOTE: atoi takes ownership of its argument; `$str` passes a copy
    var str: string = "100"
    var parsed_int: int = atoi($str)

    println(f"String \"{str}\" as integer: {parsed_int}")
    
    // Float to string
    var pi: float = 3.14159
    var str_float: string = ftoa(pi)
    
    println(f"Float {pi} as string: {str_float}")
    
    // String to float
    var float_str: string = "2.718"
    var parsed_float: float = atof($float_str)
    
    println(f"String \"{float_str}\" as float: {parsed_float}")
    
    // Arithmetic on converted values
    var sum: int = parsed_int + parsed_int
    println(f"Sum of parsed integers: {sum}")
    
    return 0
}



