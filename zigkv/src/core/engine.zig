const std = @import("std");
const command = @import("command.zig");
const Store = @import("store.zig").Store;
const response = @import("response.zig");

pub const Engine = struct {
    store: *Store,

    pub fn init(store: *Store) Engine {
        return .{ .store = store };
    }

    pub fn executeAt(
        self: *Engine,
        allocator: std.mem.Allocator,
        cmd: command.Command,
        now_ms: i64,
    ) ![]u8 {
        return switch (cmd) {
            .ping => try allocator.dupe(u8, "+PONG\r\n"),

            .get => |key| blk: {
                if (self.store.getAt(key, now_ms)) |value| {
                    break :blk try response.bulk(allocator, value);
                }
                break :blk try allocator.dupe(u8, "$nil\r\n");
            },

            .set => |args| blk: {
                try self.store.setAt(args.key, args.value, now_ms, null);
                break :blk try response.ok(allocator);
            },

            .del => |key| blk: {
                const deleted = self.store.delete(key);
                break :blk try std.fmt.allocPrint(
                    allocator,
                    ":{d}\r\n",
                    .{@intFromBool(deleted)},
                );
            },

            .exists => |key| blk: {
                const found = self.store.existsAt(key, now_ms);
                break :blk try std.fmt.allocPrint(
                    allocator,
                    ":{d}\r\n",
                    .{@intFromBool(found)},
                );
            },

            .setex => |args| blk: {
                try self.store.setAt(args.key, args.value, now_ms, args.ttl_ms);
                break :blk try response.ok(allocator);
            },
        };
    }
};

test "engine ping" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    var engine = Engine.init(&store);

    const cmd = try command.parse("PING");
    const resp = try engine.executeAt(std.testing.allocator, cmd, 0);
    defer std.testing.allocator.free(resp);

    try std.testing.expectEqualStrings("+PONG\r\n", resp);
}

test "engine set and get" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    var engine = Engine.init(&store);

    {
        const cmd = try command.parse("SET name zigkv");
        const resp = try engine.executeAt(std.testing.allocator, cmd, 0);
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings("+OK\r\n", resp);
    }

    {
        const cmd = try command.parse("GET name");
        const resp = try engine.executeAt(std.testing.allocator, cmd, 0);
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings("$zigkv\r\n", resp);
    }
}

test "engine get missing key" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    var engine = Engine.init(&store);

    const cmd = try command.parse("GET missing");
    const resp = try engine.executeAt(std.testing.allocator, cmd, 0);
    defer std.testing.allocator.free(resp);

    try std.testing.expectEqualStrings("$nil\r\n", resp);
}

test "engine setex expires" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    var engine = Engine.init(&store);

    {
        const cmd = try command.parse("SETEX tmp 10 value");
        const resp = try engine.executeAt(std.testing.allocator, cmd, 1000);
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings("+OK\r\n", resp);
    }

    {
        const cmd = try command.parse("GET tmp");
        const resp = try engine.executeAt(std.testing.allocator, cmd, 1009);
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings("$value\r\n", resp);
    }

    {
        const cmd = try command.parse("GET tmp");
        const resp = try engine.executeAt(std.testing.allocator, cmd, 1010);
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings("$nil\r\n", resp);
    }
}

test "engine exists and delete" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    var engine = Engine.init(&store);

    {
        const cmd = try command.parse("EXISTS name");
        const resp = try engine.executeAt(std.testing.allocator, cmd, 0);
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings(":0\r\n", resp);
    }

    {
        const cmd = try command.parse("SET name zigkv");
        const resp = try engine.executeAt(std.testing.allocator, cmd, 0);
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings("+OK\r\n", resp);
    }

    {
        const cmd = try command.parse("EXISTS name");
        const resp = try engine.executeAt(std.testing.allocator, cmd, 0);
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings(":1\r\n", resp);
    }

    {
        const cmd = try command.parse("DEL name");
        const resp = try engine.executeAt(std.testing.allocator, cmd, 0);
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings(":1\r\n", resp);
    }

    {
        const cmd = try command.parse("DEL name");
        const resp = try engine.executeAt(std.testing.allocator, cmd, 0);
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings(":0\r\n", resp);
    }
}
