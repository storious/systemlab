const std = @import("std");

pub const StoreError = error{
    KeyNotFound,
};

pub const Entry = struct {
    value: []u8,
    expires_at_ms: ?i64 = null,
};

pub const Store = struct {
    allocator: std.mem.Allocator,
    map: std.StringHashMap(Entry),

    pub fn init(allocator: std.mem.Allocator) Store {
        return .{
            .allocator = allocator,
            .map = std.StringHashMap(Entry).init(allocator),
        };
    }

    pub fn deinit(self: *Store) void {
        var it = self.map.iterator();
        while (it.next()) |kv| {
            self.allocator.free(kv.key_ptr.*);
            self.allocator.free(kv.value_ptr.value);
        }
        self.map.deinit();
    }

    pub fn set(self: *Store, key: []const u8, value: []const u8, ttl_ms: ?i64) !void {
        return self.setAt(key, value, 0, ttl_ms);
    }

    pub fn setAt(
        self: *Store,
        key: []const u8,
        value: []const u8,
        now_ms: i64,
        ttl_ms: ?i64,
    ) !void {
        const expires_at_ms = if (ttl_ms) |ttl| now_ms + ttl else null;

        if (self.map.fetchRemove(key)) |old| {
            self.allocator.free(old.key);
            self.allocator.free(old.value.value);
        }

        const key_copy = try self.allocator.dupe(u8, key);
        const value_copy = try self.allocator.dupe(u8, value);

        try self.map.put(key_copy, .{
            .value = value_copy,
            .expires_at_ms = expires_at_ms,
        });
    }

    pub fn get(self: *Store, key: []const u8) ?[]const u8 {
        return self.getAt(key, 0);
    }

    pub fn getAt(self: *Store, key: []const u8, now_ms: i64) ?[]const u8 {
        const entry = self.map.getPtr(key) orelse return null;

        if (entry.expires_at_ms) |deadline| {
            if (now_ms >= deadline) {
                _ = self.delete(key);
                return null;
            }
        }

        return entry.value;
    }

    pub fn exists(self: *Store, key: []const u8) bool {
        return self.existsAt(key, 0);
    }

    pub fn existsAt(self: *Store, key: []const u8, now_ms: i64) bool {
        return self.getAt(key, now_ms) != null;
    }

    pub fn delete(self: *Store, key: []const u8) bool {
        if (self.map.fetchRemove(key)) |old| {
            self.allocator.free(old.key);
            self.allocator.free(old.value.value);
            return true;
        }
        return false;
    }
};

test "set and get" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try store.set("name", "zigkv", null);

    const value = store.get("name").?;
    try std.testing.expectEqualStrings("zigkv", value);
}

test "delete key" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try store.set("a", "1", null);
    try std.testing.expect(store.delete("a"));
    try std.testing.expect(store.get("a") == null);
}

test "ttl expires" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try store.setAt("tmp", "value", 1000, 10);

    try std.testing.expectEqualStrings("value", store.getAt("tmp", 1009).?);
    try std.testing.expect(store.getAt("tmp", 1010) == null);
}

test "overwrite key replaces value" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try store.set("name", "old", null);
    try store.set("name", "new", null);

    try std.testing.expectEqualStrings("new", store.get("name").?);
}

test "overwrite ttl key with persistent value" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try store.setAt("k", "ttl", 1000, 10);
    try store.setAt("k", "persist", 1005, null);

    try std.testing.expectEqualStrings("persist", store.getAt("k", 2000).?);
}

test "overwrite persistent key with ttl value" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try store.setAt("k", "persist", 1000, null);
    try store.setAt("k", "ttl", 1005, 10);

    try std.testing.expectEqualStrings("ttl", store.getAt("k", 1014).?);
    try std.testing.expect(store.getAt("k", 1015) == null);
}

test "exists returns false after ttl expiration" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try store.setAt("tmp", "value", 1000, 10);

    try std.testing.expect(store.existsAt("tmp", 1009));
    try std.testing.expect(!store.existsAt("tmp", 1010));
}

test "delete missing key returns false" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try std.testing.expect(!store.delete("missing"));
}
