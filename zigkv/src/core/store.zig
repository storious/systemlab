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

    pub fn clear(self: *Store) void {
        var it = self.map.iterator();
        while (it.next()) |kv| {
            self.allocator.free(kv.key_ptr.*);
            self.allocator.free(kv.value_ptr.value);
        }
        self.map.clearRetainingCapacity();
    }

    pub fn ttlAt(self: *Store, key: []const u8, now_ms: i64) i64 {
        const entry = self.map.getPtr(key) orelse return -2;

        if (entry.expires_at_ms) |deadline| {
            if (now_ms >= deadline) {
                _ = self.delete(key);
                return -2;
            }

            return deadline - now_ms;
        }

        return -1;
    }

    pub fn persistAt(self: *Store, key: []const u8, now_ms: i64) bool {
        const entry = self.map.getPtr(key) orelse return false;

        if (entry.expires_at_ms) |deadline| {
            if (now_ms >= deadline) {
                _ = self.delete(key);
                return false;
            }

            entry.expires_at_ms = null;
            return true;
        }

        return false;
    }

    fn isExpired(entry: *const Entry, now_ms: i64) bool {
        if (entry.expires_at_ms) |deadline| {
            return now_ms >= deadline;
        }
        return false;
    }

    pub fn keysAt(self: *Store, allocator: std.mem.Allocator, now_ms: i64) ![][]u8 {
        var expired = std.ArrayList([]u8){
            .items = &.{},
            .capacity = 0,
        };
        defer {
            for (expired.items) |key| {
                allocator.free(key);
            }
            expired.deinit(allocator);
        }

        var out = std.ArrayList([]u8){
            .items = &.{},
            .capacity = 0,
        };
        errdefer {
            for (out.items) |key| {
                allocator.free(key);
            }
            out.deinit(allocator);
        }

        var it = self.map.iterator();
        while (it.next()) |kv| {
            if (isExpired(kv.value_ptr, now_ms)) {
                const key_copy = try allocator.dupe(u8, kv.key_ptr.*);
                try expired.append(allocator, key_copy);
                continue;
            }

            const key_copy = try allocator.dupe(u8, kv.key_ptr.*);
            try out.append(allocator, key_copy);
        }

        for (expired.items) |key| {
            _ = self.delete(key);
        }
        std.mem.sort([]u8, out.items, {}, lessThan);
        return out.toOwnedSlice(allocator);
    }

    fn lessThan(_: void, a: []u8, b: []u8) bool {
        return std.mem.lessThan(u8, a, b);
    }

    pub fn keys(self: *Store, allocator: std.mem.Allocator) ![][]u8 {
        return self.keysAt(allocator, 0);
    }

    pub fn freeKeys(allocator: std.mem.Allocator, keys_slice: [][]u8) void {
        for (keys_slice) |key| {
            allocator.free(key);
        }
        allocator.free(keys_slice);
    }

    pub fn len(self: *const Store) usize {
        return self.map.count();
    }

    pub fn isEmpty(self: *const Store) bool {
        return self.len() == 0;
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

test "store reports length" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try std.testing.expect(store.isEmpty());
    try std.testing.expectEqual(@as(usize, 0), store.len());

    try store.set("a", "1", null);
    try store.set("b", "2", null);

    try std.testing.expect(!store.isEmpty());
    try std.testing.expectEqual(@as(usize, 2), store.len());

    _ = store.delete("a");

    try std.testing.expectEqual(@as(usize, 1), store.len());
}

test "clear removes all keys" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try store.set("a", "1", null);
    try store.set("b", "2", null);

    try std.testing.expectEqual(@as(usize, 2), store.len());

    store.clear();

    try std.testing.expect(store.isEmpty());
    try std.testing.expect(store.get("a") == null);
    try std.testing.expect(store.get("b") == null);
}

test "ttlAt reports remaining ttl" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try store.setAt("tmp", "value", 1000, 10);

    try std.testing.expectEqual(@as(i64, 10), store.ttlAt("tmp", 1000));
    try std.testing.expectEqual(@as(i64, 1), store.ttlAt("tmp", 1009));
    try std.testing.expectEqual(@as(i64, -2), store.ttlAt("tmp", 1010));
}

test "ttlAt reports persistent and missing keys" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try store.setAt("persist", "value", 1000, null);

    try std.testing.expectEqual(@as(i64, -1), store.ttlAt("persist", 1000));
    try std.testing.expectEqual(@as(i64, -2), store.ttlAt("missing", 1000));
}

test "persistAt removes ttl" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try store.setAt("tmp", "value", 1000, 10);

    try std.testing.expectEqual(@as(i64, 10), store.ttlAt("tmp", 1000));
    try std.testing.expect(store.persistAt("tmp", 1005));
    try std.testing.expectEqual(@as(i64, -1), store.ttlAt("tmp", 2000));
    try std.testing.expectEqualStrings("value", store.getAt("tmp", 2000).?);
}

test "persistAt returns false for missing persistent and expired keys" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try std.testing.expect(!store.persistAt("missing", 1000));

    try store.setAt("persist", "value", 1000, null);
    try std.testing.expect(!store.persistAt("persist", 1000));

    try store.setAt("expired", "value", 1000, 10);
    try std.testing.expect(!store.persistAt("expired", 1010));
}

test "keys returns stored keys" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try store.set("a", "1", null);
    try store.set("b", "2", null);

    const ks = try store.keys(std.testing.allocator);
    defer Store.freeKeys(std.testing.allocator, ks);

    try std.testing.expectEqual(@as(usize, 2), ks.len);
}

test "keysAt excludes and removes expired keys" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try store.setAt("alive", "1", 1000, 20);
    try store.setAt("expired", "2", 1000, 10);

    const ks = try store.keysAt(std.testing.allocator, 1010);
    defer Store.freeKeys(std.testing.allocator, ks);

    try std.testing.expectEqual(@as(usize, 1), ks.len);
    try std.testing.expectEqualStrings("alive", ks[0]);
    try std.testing.expectEqual(@as(usize, 1), store.len());
    try std.testing.expect(store.getAt("expired", 1010) == null);
}

test "keysAt returns keys in sorted order" {
    var store = Store.init(std.testing.allocator);
    defer store.deinit();

    try store.set("b", "2", null);
    try store.set("a", "1", null);

    const ks = try store.keysAt(std.testing.allocator, 0);
    defer Store.freeKeys(std.testing.allocator, ks);

    try std.testing.expectEqualStrings("a", ks[0]);
    try std.testing.expectEqualStrings("b", ks[1]);
}
