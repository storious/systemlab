const std = @import("std");

const Store = @import("../core/store.zig").Store;
const engine = @import("../core/engine.zig");
const clock = @import("../core/clock.zig");
const protocol = @import("../core/protocol.zig");

pub const App = struct {
    allocator: std.mem.Allocator,
    store: Store,
    now_ms: i64,

    pub fn init(allocator: std.mem.Allocator) App {
        return .{
            .allocator = allocator,
            .store = Store.init(allocator),
            .now_ms = 0,
        };
    }

    pub fn setNow(self: *App, now_ms: i64) void {
        self.now_ms = now_ms;
    }

    pub fn deinit(self: *App) void {
        self.store.deinit();
    }

    pub fn executeText(self: *App, input: []const u8) ![]u8 {
        const cmd = protocol.parse(input) catch |err| {
            return protocol.formatError(self.allocator, err);
        };

        var exec = engine.Engine.init(&self.store);
        return exec.executeAt(self.allocator, cmd, self.now_ms);
    }

    pub fn len(self: *const App) usize {
        return self.store.len();
    }

    pub fn isEmpty(self: *const App) bool {
        return self.store.isEmpty();
    }
};

test "app executes ping" {
    var app = App.init(std.testing.allocator);
    defer app.deinit();

    const resp = try app.executeText("PING");
    defer std.testing.allocator.free(resp);

    try std.testing.expectEqualStrings("+PONG\r\n", resp);
}

test "app returns parse error response" {
    var app = App.init(std.testing.allocator);
    defer app.deinit();

    const resp = try app.executeText("UNKNOWN");
    defer std.testing.allocator.free(resp);

    try std.testing.expectEqualStrings("-ERR UnknownCommand\r\n", resp);
}

test "app executes set and get in same instance" {
    var app = App.init(std.testing.allocator);
    defer app.deinit();

    {
        const resp = try app.executeText("SET name zigkv");
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings("+OK\r\n", resp);
    }

    {
        const resp = try app.executeText("GET name");
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings("$zigkv\r\n", resp);
    }
}

test "app respects logical time for ttl" {
    var app = App.init(std.testing.allocator);
    defer app.deinit();

    app.setNow(1000);
    {
        const resp = try app.executeText("SETEX tmp 10 value");
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings("+OK\r\n", resp);
    }

    app.setNow(1009);
    {
        const resp = try app.executeText("GET tmp");
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings("$value\r\n", resp);
    }

    app.setNow(1010);
    {
        const resp = try app.executeText("GET tmp");
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings("$nil\r\n", resp);
    }
}

test "app exposes store size" {
    var app = App.init(std.testing.allocator);
    defer app.deinit();

    try std.testing.expect(app.isEmpty());
    try std.testing.expectEqual(@as(usize, 0), app.len());

    {
        const resp = try app.executeText("SET a 1");
        defer std.testing.allocator.free(resp);
    }

    try std.testing.expect(!app.isEmpty());
    try std.testing.expectEqual(@as(usize, 1), app.len());
}

test "app clears store" {
    var app = App.init(std.testing.allocator);
    defer app.deinit();

    {
        const resp = try app.executeText("SET a 1");
        defer std.testing.allocator.free(resp);
    }

    try std.testing.expectEqual(@as(usize, 1), app.len());

    {
        const resp = try app.executeText("CLEAR");
        defer std.testing.allocator.free(resp);
        try std.testing.expectEqualStrings("+OK\r\n", resp);
    }

    try std.testing.expect(app.isEmpty());
}
