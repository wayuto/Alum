import io
using io::println

fun main(): int {
    // continue in while loop
    println("continue in while loop:");
    var i: int = 0;
    while i < 10 {
        i = i + 1;
        if i == 5 {
            continue;
        }
        println(f"{i}");
    }
    println("Done!");

    // continue in for loop
    println("continue in for loop:");
    for j in 0..10 {
        if j == 5 {
            continue;
        }
        println(f"{j}");
    }
    println("Done!");

    return 0;
}



