$import "io.ah"
$import "convert.ah"
$import "vec.ah"

// Vector Example

fun main(): int {
	let v: Vec<int> = vec_new()
	for i in 0..10 {
		v.push(&v, i * i)
	}
	
	for i in 0..10 {
		println(itoa(v[i]))
	}
	return 0
}