//! By convention, root.zig is the root source file when making a package.
const std = @import("std");
const Io = std.Io;

/// This is a documentation comment to explain the `printAnotherMessage` function below.
///
/// Accepting an `Io.Writer` instance is a handy way to write reusable code.
pub fn printAnotherMessage(writer: *Io.Writer) Io.Writer.Error!void {
    try writer.print("Run `zig build test` to run the tests.\n", .{});
}

pub fn add(a: i32, b: i32) i32 {
    return a + b;
}

test "basic add functionality" {
    try std.testing.expect(add(3, 7) == 10);
}

pub const Store = @import("core/store.zig").Store;
pub const command = @import("core/command.zig");
pub const engine = @import("core/engine.zig");
pub const response = @import("core/response.zig");
pub const clock = @import("core/clock.zig");

test {
    _ = @import("store.zig");
    _ = @import("command.zig");
    _ = @import("engine.zig");
    _ = @import("clock.zig");
    _ = @import("response.zig");
}
