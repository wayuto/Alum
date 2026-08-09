const std = @import("std");

fn factorial(n: u64) u64 {
    if (n < 2) return 1;
    return n * factorial(n - 1);
}

fn itoa(buf: *[24]u8, init_n: u64) []const u8 {
    var idx: usize = buf.len;
    var n = init_n;
    if (n == 0) {
        buf[buf.len - 1] = '0';
        return buf[buf.len - 1 ..];
    }
    while (n > 0) {
        idx -= 1;
        buf[idx] = @intCast('0' + @as(u8, @intCast(n % 10)));
        n /= 10;
    }
    return buf[idx..];
}

const RESULT: u64 = factorial(20);

pub fn main() !void {
    var buf = [_]u8{0} ** 24;
    const s = itoa(&buf, RESULT);
    _ = std.os.linux.write(std.posix.STDOUT_FILENO, s.ptr, s.len);
    _ = std.os.linux.write(std.posix.STDOUT_FILENO, "\n", 1);
}