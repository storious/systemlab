const std = @import("std");
const zigkv = @import("zigkv");

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

pub fn main(init: std.process.Init) !void {
    var gpa = std.heap.DebugAllocator(.{}){};
    defer _ = gpa.deinit();

    const allocator = gpa.allocator();

    const input = try collectInput(allocator, init);
    defer allocator.free(input);

    var store = zigkv.Store.init(allocator);
    defer store.deinit();

    var engine = zigkv.engine.Engine.init(&store);
    const clock = zigkv.clock.Clock.fixed(0);

    const cmd = zigkv.command.parse(input) catch |err| {
        std.debug.print("-ERR {s}\r\n", .{@errorName(err)});
        return;
    };

    const resp = engine.executeAt(allocator, cmd, clock.now()) catch |err| {
        std.debug.print("-ERR {s}\r\n", .{@errorName(err)});
        return;
    };
    defer allocator.free(resp);

    try std.Io.File.stdout().writeStreamingAll(init.io, resp);
}

test "simple test" {
    const gpa = std.testing.allocator;
    var list: std.ArrayList(i32) = .empty;
    defer list.deinit(gpa); // Try commenting this out and see if zig detects the memory leak!
    try list.append(gpa, 42);
    try std.testing.expectEqual(@as(i32, 42), list.pop());
}

test "fuzz example" {
    try std.testing.fuzz({}, testOne, .{});
}

fn testOne(context: void, smith: *std.testing.Smith) !void {
    _ = context;
    // Try passing `--fuzz` to `zig build test` and see if it manages to fail this test case!

    const gpa = std.testing.allocator;
    var list: std.ArrayList(u8) = .empty;
    defer list.deinit(gpa);
    while (!smith.eos()) switch (smith.value(enum { add_data, dup_data })) {
        .add_data => {
            const slice = try list.addManyAsSlice(gpa, smith.value(u4));
            smith.bytes(slice);
        },
        .dup_data => {
            if (list.items.len == 0) continue;
            if (list.items.len > std.math.maxInt(u32)) return error.SkipZigTest;
            const len = smith.valueRangeAtMost(u32, 1, @min(32, list.items.len));
            const off = smith.valueRangeAtMost(u32, 0, @intCast(list.items.len - len));
            try list.appendSlice(gpa, list.items[off..][0..len]);
            try std.testing.expectEqualSlices(
                u8,
                list.items[off..][0..len],
                list.items[list.items.len - len ..],
            );
        },
    };
}
