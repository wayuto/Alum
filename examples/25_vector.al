import io
using io::{println, print}
import maybe
using maybe::Maybe
import vec
using vec::{Vec, vec_new}

// Vector Example

fun main(): int {
	var v: Vec<int> = vec_new()
	for i in 0..10 {
		v.push(&v, i * i)
	}
	
	for i in v {
		println(f"{i}")
	}

	return 0
}