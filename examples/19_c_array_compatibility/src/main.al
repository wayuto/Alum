import io
using io::println
import array_compat
using array_compat::{c_array_sum, c_array_max, c_array_reverse}

// C Array Compatibility Example
// Alum arrays are now fully compatible with C arrays
// They have the same memory layout (just data, no headers)

fun main(): int {
    // Create an Alum array
    var alum_arr: int[5] = [1, 2, 3, 4, 5]

    // Pass Alum array directly to C function
    // No conversion needed - arrays are fully compatible!
    var sum: int = c_array_sum(alum_arr, 5)
    println(f"Sum of Alum array [1,2,3,4,5] computed by C: {sum}")

    // Find max using C function
    var max_val: int = c_array_max(alum_arr, 5)
    println(f"Max of Alum array computed by C: {max_val}")

    return 0
}



