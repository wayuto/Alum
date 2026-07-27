$import "io.ah"
$import "convert.ah"

// Getting args from command-line Example

fun main(argc: int, argv: string[]): int {
    print("Program Name: ")
    // arg[0] = Executable file path
    println(argv[0])

    for i in 1..argc {
        print("Arg ")
        print(itoa(i))
        print(": ")
        println(argv[i])
    }
    return 0
}