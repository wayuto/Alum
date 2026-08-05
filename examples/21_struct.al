$import "io.ah"

// Struct declaration and using Example

struct Point {
    x: int, 
    y: int
}

fun main(): int {
    var p: Point = Point {
        x: 1,
        y: 1
    }
    println(f"Point({p.x}, {p.y})")
    return 0
}
