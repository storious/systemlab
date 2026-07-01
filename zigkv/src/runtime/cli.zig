const std = @import("std");

const Store = @import("../core/store.zig").Store;
const Command = @import("../core/command.zig");
const Engine = @import("../core/engine.zig");
const Response = @import("../core/response.zig");
const Clock = @import("../core/clock.zig");

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

    var store = Store.init(allocator);
    defer store.deinit();

    var engine = Engine.Engine.init(&store);
    const clock = Clock.Clock.fixed(0);

    const cmd = Command.parse(input) catch |err| {
        const resp = try Response.err(allocator, @errorName(err));
        defer allocator.free(resp);
        try std.Io.File.stdout().writeStreamingAll(init.io, resp);
        return;
    };

    const resp = engine.executeAt(allocator, cmd, clock.now()) catch |err| {
        const err_resp = try Response.err(allocator, @errorName(err));
        defer allocator.free(err_resp);
        try std.Io.File.stdout().writeStreamingAll(init.io, err_resp);
        return;
    };
    defer allocator.free(resp);

    try std.Io.File.stdout().writeStreamingAll(init.io, resp);
}
