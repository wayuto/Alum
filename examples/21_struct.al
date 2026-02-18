$import "io.al"
$import "convert.al"

// Struct declaration and using Example

struct Point {
    x: int, 
    y: int
}

fun main(): int {
    let p: Point = Point {
        x: 1,
        y: 1
    }
    println("Point(" + itoa(p.x) + ", " + itoa(p.y) + ")")
    return 0
}