const std = @import("std");

fn sort(arr: *[10]i32) void {
    const n: usize = arr.len;
    var j: usize = 0;
    while (j < n - 1) : (j += 1) {
        var k: usize = 0;
        while (k < n - j - 1) : (k += 1) {
            if (arr[k] > arr[k + 1]) {
                const t = arr[k];
                arr[k] = arr[k + 1];
                arr[k + 1] = t;
            }
        }
    }
}

fn itoa(buf: *[16]u8, init_n: i32) []const u8 {
    var idx: usize = buf.len;
    var n = init_n;
    if (n == 0) {
        buf[buf.len - 1] = '0';
        return buf[buf.len - 1 ..];
    }
    while (n > 0) {
        idx -= 1;
        buf[idx] = @intCast('0' + @as(u8, @intCast(@rem(n, 10))));
        n = @divTrunc(n, 10);
    }
    return buf[idx..];
}

pub fn main() !void {
    var arr = [10]i32{ 9, 2, 7, 1, 8, 3, 6, 4, 10, 5 };
    sort(&arr);
    var buf = [_]u8{0} ** 16;
    for (arr) |x| {
        const s = itoa(&buf, x);
        _ = std.os.linux.write(std.posix.STDOUT_FILENO, s.ptr, s.len);
        _ = std.os.linux.write(std.posix.STDOUT_FILENO, "\n", 1);
    }
}