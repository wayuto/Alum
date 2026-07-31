$import "vec.ah"

fun vec_new(): Vec<int> {
    return Vec<int> {
        data: [int; 0],
        len: 0,
        capacity: 0,
		at: \(v: *Vec<int>, i: int): int {
			if i >= v.len || i < 0 return 0
			return v.data[i]
		},
		push: \(v: *Vec<int>, elem: int): void {
			if v.len >= v.capacity {
				let new_capacity: int = if v.capacity == 0 {
					4
				} else {
					v.capacity * 2
				}
				let new_data: int[] = [int; new_capacity]
				for i in 0..v.len {
					new_data[i] = v.data[i]
				}
				v.data = new_data
				v.capacity = new_capacity
			}
			v.data[v.len] = elem
			v.len = v.len + 1
		},
		pop: \(v: *Vec<int>): int {
			if v.len == 0 return 0
			v.len = v.len - 1
			let elem: int = v.data[v.len]
			return elem
		},
		clear: \(v: *Vec<int>): void {
			v.len = 0
			v.capacity = 0
			v.data = [int; 0]
		},
	}
}
