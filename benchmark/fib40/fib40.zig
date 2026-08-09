const std = @import("std");

fn fib(n: u64) u64 {
    if (n < 2) return n;
    return fib(n - 1) + fib(n - 2);
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

const RESULT: u64 = blk: {
    @setEvalBranchQuota(1000000000);
    break :blk fib(40);
};

pub fn main() !void {
    var buf = [_]u8{0} ** 24;
    const s = itoa(&buf, RESULT);
    _ = std.os.linux.write(std.posix.STDOUT_FILENO, s.ptr, s.len);
    _ = std.os.linux.write(std.posix.STDOUT_FILENO, "\n", 1);
}