$import "io.al"
$import "convert.al"

fun main(argc: int, argv: arr[string]): int {
    print("Program Name: ")
    println(argv[0])
    for i in 1..argc {
        print("Arg")
        print(itoa(i))
        print(": ")
        println(argv[i])
    }
    return 0
}