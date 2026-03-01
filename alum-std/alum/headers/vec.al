$ifndef ALUM_VEC
$define ALUM_VEC 1

struct Vec {
    data: gen[],
    len: int,
    capacity: int,
    at: gen(*Vec, int),
    push: void(*Vec, gen),
    pop: gen(*Vec),
    clear: void(*Vec),
}

extern vec_new(): Vec

$endif