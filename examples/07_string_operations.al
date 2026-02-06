$import "io.al"
$import "string.al"

// String Operations Example
// Demonstrates string manipulation

fun main(): int {
    let hello: string = "Hello"
    let world: string = "World"
    let target: string = ""
    
    // String concatenation using strcat
    target = strcat(hello, ", ")
    target = strcat(target, world)
    target = strcat(target, "!")
    
    print("Concatenated: ")
    println(target)
    
    // String length
    let len: int = strlen(target)
    print("Length: ")
    println(itoa(len))
    
    // String copy
    let copy: string = ""
    copy = strcpy(copy, target)
    
    print("Copy: ")
    println(copy)
    
    // String comparison
    let result: int = strcmp(target, copy)
    if result == 0 {
        println("Strings are equal")
    } else {
        println("Strings are different")
    }
    
    // Different string
    let other: string = "Goodbye"
    let diff: int = strcmp(target, other)
    if diff == 0 {
        println("Strings are equal")
    } else {
        println("Strings are different")
    }
    
    return 0
}

// Simple string comparison function
fun strcmp(s1: string, s2: string): int {
    let i: int = 0
    while s1[i] != '\0' && s2[i] != '\0' {
        if s1[i] != s2[i] {
            return s1[i] - s2[i]
        }
        i = i + 1
    }
    return s1[i] - s2[i]
}