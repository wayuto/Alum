$import "io.ah"

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
    println(f"RED = {Color.RED}")      // 0
    println(f"GREEN = {Color.GREEN}")    // 5
    println(f"BLUE = {Color.BLUE}")     // 6
    println(f"WHITE = {Color.WHITE}")    // 11

    // Bare (C-style) member reference
    println(f"NORTH = {NORTH}")          // -1

    // Enums are ints and work in expressions
    let next: int = Color.BLACK + 1
    println(f"BLACK + 1 = {next}")

    // Enums as function arguments and comparisons
    println(f"color_name(BLUE) = {color_name(Color.BLUE)}")

    return 0
}
