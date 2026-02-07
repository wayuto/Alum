$import "array.al"

fun main(): int {
    // Test range function with C-compatible layout
    let arr: arr[int] = range(0, 5);
    
    // Access elements directly (C-compatible)
    let x: int = arr[0];
    let y: int = arr[1];
    let z: int = arr[4];
    
    return 0;
}