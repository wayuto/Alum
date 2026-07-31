$import "io.ah"
$import "convert.ah"
$import "array_compat.ah"

// C Array Compatibility Example
// Alum arrays are now fully compatible with C arrays
// They have the same memory layout (just data, no headers)

fun main(): int {
    // Create an Alum array
    let alum_arr: int[5] = [1, 2, 3, 4, 5]

    // Pass Alum array directly to C function
    // No conversion needed - arrays are fully compatible!
    let sum: int = c_array_sum(alum_arr, 5)
    print("Sum of Alum array [1,2,3,4,5] computed by C: ")
    println(itoa(sum))

    // Find max using C function
    let max_val: int = c_array_max(alum_arr, 5)
    print("Max of Alum array computed by C: ")
    println(itoa(max_val))

    return 0
}
