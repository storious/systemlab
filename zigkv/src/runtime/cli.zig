const std = @import("std");
const App = @import("app.zig").App;

fn collectInput(
    allocator: std.mem.Allocator,
    init: std.process.Init,
) ![]u8 {
    const args = try init.minimal.args.toSlice(init.arena.allocator());

    if (args.len <= 1) {
        return allocator.dupe(u8, "PING");
    }

    return std.mem.join(allocator, " ", args[1..]);
}

pub fn run(init: std.process.Init) !void {
    var gpa = std.heap.DebugAllocator(.{}){};
    defer _ = gpa.deinit();

    const allocator = gpa.allocator();

    const input = try collectInput(allocator, init);
    defer allocator.free(input);

    var app = App.init(allocator);
    defer app.deinit();

    const resp = try app.executeText(input);
    defer allocator.free(resp);

    try std.Io.File.stdout().writeStreamingAll(init.io, resp);
}
