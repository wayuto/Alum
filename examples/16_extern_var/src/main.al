import vars
import io
using io::{write, println}

// Alum reading and writing C extern global variables.
// The C file `src/vars.c` defines `counter` and `ratio`.

fun main(): int {
    var initial: int = counter
    println(f"initial counter = {initial}")

    counter = 42
    counter += 1
    ++counter
    println(f"counter after writes = {counter}")

    var r0: float = ratio
    println(f"ratio = {r0}")

    ratio = 2.5
    println(f"ratio after write = {ratio}")

    var twice: float = ratio * 2.0
    println(f"ratio * 2 = {twice}")

    --counter
    counter -= 5
    println(f"counter at end = {counter}")

    return 0
}



