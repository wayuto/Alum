extern range(int int): arr<_>

pub fun find(a: arr<_> e: int): int {
    let i: int = 0
    while i < sizeof a {
        if a[i] == e {
            return i
        }
        i++
    }
    return (-1)
}