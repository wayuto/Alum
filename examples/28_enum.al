$import "io.ah"
$import "convert.ah"

// C-style enum with auto-incrementing and explicit values

enum Color {
    RED,
    GREEN = 5,
    BLUE,
    BLACK = 10,
    WHITE
}

enum Direction {
    NORTH = -1,
    EAST = 0,
    SOUTH = 1
}

fun color_name(c: Color): string {
    if c == Color.RED {
        return "red"
    } else if c == Color.GREEN {
        return "green"
    } else if c == Color.BLUE {
        return "blue"
    } else {
        return "other"
    }
}

fun main(): int {
    // Values auto-increment unless explicitly set
    print("RED = ")
    println(itoa(Color.RED))      // 0
    print("GREEN = ")
    println(itoa(Color.GREEN))    // 5
    print("BLUE = ")
    println(itoa(Color.BLUE))     // 6
    print("WHITE = ")
    println(itoa(Color.WHITE))    // 11

    // Bare (C-style) member reference
    print("NORTH = ")
    println(itoa(NORTH))          // -1

    // Enums are ints and work in expressions
    let next: int = Color.BLACK + 1
    print("BLACK + 1 = ")
    println(itoa(next))

    // Enums as function arguments and comparisons
    print("color_name(BLUE) = ")
    println(color_name(Color.BLUE))

    return 0
}
