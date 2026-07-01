const std = @import("std");

pub const Clock = struct {
    now_ms: i64,

    pub fn fixed(now_ms: i64) Clock {
        return .{ .now_ms = now_ms };
    }

    pub fn now(self: Clock) i64 {
        return self.now_ms;
    }
};

test "fixed clock" {
    const clock = Clock.fixed(1234);
    try std.testing.expectEqual(@as(i64, 1234), clock.now());
}
