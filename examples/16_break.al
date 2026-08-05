$import "io.ah"

fun main(): int {
    // break in while loop
    println("break in while loop:");
    var i: int = 0;
    while i < 10 {
        if i == 5 {
            break;
        }
        println(f"{i}");
        i = i + 1;
    }
    println("Done!");

    // break in for loop
    println("break in for loop:");
    for j in 0..10 {
        if j == 5 {
            break;
        }
        println(f"{j}");
    }
    println("Done!");

    return 0;
}