$import "io.ah"
$import "maybe.ah"
$import "vec.ah"

// Vector Example

fun main(): int {
	let v: Vec<int> = vec_new()
	for i in 0..10 {
		v.push(&v, i * i)
	}
	
	for i in 0..10 {
		let m: Maybe<int> = v[i]
		if m.tag == Just {
			println(f"{m.value}")
		} else {
			println("out of bounds")
		}
	}
	return 0
}