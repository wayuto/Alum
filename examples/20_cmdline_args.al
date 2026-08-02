$import "io.ah"

// Getting args from command-line Example

fun main(argc: int, argv: string[]): int {
    println(f"Program Name: {argv[0]}")

    for i in 1..argc {
        println(f"Arg {i}: {argv[i]}")
    }
    return 0
}