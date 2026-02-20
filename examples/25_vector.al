$import "io.al"
$import "convert.al"
$import "vec.al"

// Vector Example

fun main(): int {
	let v: Vec = vec_new()
	for i in 0..10 {
		v.push(&v, i * i)
	}
	
	for i in 0..10 {
		println(itoa(v.at(&v, i)))
	}
	return 0
}