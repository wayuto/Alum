$ifndef ALUM_VEC
$define ALUM_VECs 1

struct Vec {
    data: arr[any],
    len: int,
    at: any(*Vec, int),
    push: void(*Vec, any),
    pop: any(*Vec),
}

extern vec_new(): Vec

$endif