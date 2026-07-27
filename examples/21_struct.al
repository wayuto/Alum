$import "io.ah"
$import "convert.ah"

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
    println("Point(" + itoa(p.x) + ", " + itoa(p.y) +")")
    return 0
}
