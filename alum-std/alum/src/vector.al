$import "vec.al"

fun vec_new(): Vec {
    return Vec {
        data: [any; 1], 
        len: 0,
		at: lamb(v: *Vec, i: int): any return v.data[i],
		push: lamb(v: *Vec, elem: any): void {
			v.len = v.len + 1
			let data: arr[any] = [any; v.len]
			for i in 0..v.len - 1 {
				data[i] = v.data[i]
			}
			data[v.len - 1] = elem
			v.data = data
		},
		pop: lamb(v: *Vec): any {
			v.len = v.len - 1
			let data: arr[any] = [any; v.len]
			for i in 0..v.len {
				data[i] = v.data[i]
			}
			v.data = data
			return v.data[v.len]
		},
	}
}