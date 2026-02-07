$import "io.al"
$import "convert.al"
$import "array_compat.al"

// C Array Compatibility Example
// Demonstrates that Alum arrays have C-compatible memory layout

fun main(): int {
    // Create an Alum array
    let alum_arr: arr[int] = [1, 2, 3, 4, 5]
    
    // Pass it to C function to sum elements
    let sum: int = c_array_sum(alum_arr, 5)
    print("Sum of Alum array [1,2,3,4,5] computed by C: ")
    println(itoa(sum))
    
    // Create array with fill syntax
    let fill_arr: arr[int] = [int; 10]
    
    // Fill some values
    fill_arr[0] = 10
    fill_arr[1] = 20
    fill_arr[2] = 30
    
    let fill_sum: int = c_array_sum(fill_arr, 3)
    print("Sum of filled array [10,20,30] computed by C: ")
    println(itoa(fill_sum))
    
    return 0
}