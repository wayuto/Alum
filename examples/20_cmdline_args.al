$import "io.al"
$import "convert.al"

// Getting args from command-line Example

fun main(argc: int, argv: arr[string]): int {
    print("Program Name: ")
    // arg[0] = Executable file path
    println(argv[0])
    
    for i in 1..argc println("Arg" + "itoa(i)" + ": " + argv[i])
    return 0
}
