$import "vec.al"

fun vec_new(): Vec {
    return Vec {
        data: [any; 0],
        len: 0,
        capacity: 0,
		at: lamb(v: *Vec, i: int): any {
			if i >= v.len || i < 0 return nil
			return v.data[i]
		},
		push: lamb(v: *Vec, elem: any): void {
			if v.len >= v.capacity {
				let new_capacity: int = if v.capacity == 0 {
					4
				} else {
					v.capacity * 2
				}
				let new_data: arr[any] = [any; new_capacity]
				for i in 0..v.len {
					new_data[i] = v.data[i]
				}
				v.data = new_data
				v.capacity = new_capacity
			}
			v.data[v.len] = elem
			v.len = v.len + 1
		},
		pop: lamb(v: *Vec): any {
			if v.len == 0 return nil
			v.len = v.len - 1
			let elem: any = v.data[v.len]
			return elem
		},
		clear: lamb(v: *Vec): void {
			v.len = 0
			v.capacity = 0
			v.data = [any; 0]
		},
	}
}