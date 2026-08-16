import io
using io::println
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
	
	for i in 0..10 {
		var m: Maybe<int> = v[i]
		if m.tag == Just {
			println(f"{m.value}")
		} else {
			println("out of bounds")
		}
	}
	return 0
}



