const std = @import("std");

pub const CommandTag = enum {
    ping,
    get,
    set,
    del,
    exists,
    setex,
    clear,
    ttl,
    persist,
    keys,
    dbsize,
};
pub const Command = union(CommandTag) {
    ping: void,
    get: []const u8,
    set: struct {
        key: []const u8,
        value: []const u8,
    },
    del: []const u8,
    exists: []const u8,
    setex: struct {
        key: []const u8,
        ttl_ms: i64,
        value: []const u8,
    },
    clear: void,
    ttl: []const u8,
    persist: []const u8,
    keys: void,
    dbsize: void,
};

pub const ParseError = error{
    EmptyCommand,
    UnknownCommand,
    InvalidArity,
    InvalidInteger,
};

pub fn parse(input: []const u8) ParseError!Command {
    var it = std.mem.tokenizeAny(u8, input, " \t\r\n");

    const op_raw = it.next() orelse return ParseError.EmptyCommand;

    if (std.ascii.eqlIgnoreCase(op_raw, "PING")) {
        if (it.next() != null) return ParseError.InvalidArity;
        return .{ .ping = {} };
    }

    if (std.ascii.eqlIgnoreCase(op_raw, "GET")) {
        const key = it.next() orelse return ParseError.InvalidArity;
        if (it.next() != null) return ParseError.InvalidArity;
        return .{ .get = key };
    }

    if (std.ascii.eqlIgnoreCase(op_raw, "DEL")) {
        const key = it.next() orelse return ParseError.InvalidArity;
        if (it.next() != null) return ParseError.InvalidArity;
        return .{ .del = key };
    }

    if (std.ascii.eqlIgnoreCase(op_raw, "EXISTS")) {
        const key = it.next() orelse return ParseError.InvalidArity;
        if (it.next() != null) return ParseError.InvalidArity;
        return .{ .exists = key };
    }

    if (std.ascii.eqlIgnoreCase(op_raw, "SET")) {
        const key = it.next() orelse return ParseError.InvalidArity;
        const value = it.next() orelse return ParseError.InvalidArity;
        if (it.next() != null) return ParseError.InvalidArity;
        return .{ .set = .{ .key = key, .value = value } };
    }

    if (std.ascii.eqlIgnoreCase(op_raw, "SETEX")) {
        const key = it.next() orelse return ParseError.InvalidArity;
        const ttl_raw = it.next() orelse return ParseError.InvalidArity;
        const value = it.next() orelse return ParseError.InvalidArity;
        if (it.next() != null) return ParseError.InvalidArity;

        const ttl_ms = std.fmt.parseInt(i64, ttl_raw, 10) catch {
            return ParseError.InvalidInteger;
        };

        return .{ .setex = .{ .key = key, .ttl_ms = ttl_ms, .value = value } };
    }

    if (std.ascii.eqlIgnoreCase(op_raw, "CLEAR")) {
        if (it.next() != null) return ParseError.InvalidArity;
        return .{ .clear = {} };
    }

    if (std.ascii.eqlIgnoreCase(op_raw, "TTL")) {
        const key = it.next() orelse return ParseError.InvalidArity;
        if (it.next() != null) return ParseError.InvalidArity;
        return .{ .ttl = key };
    }

    if (std.ascii.eqlIgnoreCase(op_raw, "PERSIST")) {
        const key = it.next() orelse return ParseError.InvalidArity;
        if (it.next() != null) return ParseError.InvalidArity;
        return .{ .persist = key };
    }

    if (std.ascii.eqlIgnoreCase(op_raw, "KEYS")) {
        if (it.next() != null) return ParseError.InvalidArity;
        return .{ .keys = {} };
    }

    if (std.ascii.eqlIgnoreCase(op_raw, "DBSIZE")) {
        if (it.next() != null) return ParseError.InvalidArity;
        return .{ .dbsize = {} };
    }

    return ParseError.UnknownCommand;
}

test "parse ping" {
    const cmd = try parse("PING\r\n");
    try std.testing.expect(cmd == .ping);
}

test "parse get" {
    const cmd = try parse("GET name");

    switch (cmd) {
        .get => |key| try std.testing.expectEqualStrings("name", key),
        else => return error.UnexpectedCommand,
    }
}

test "parse set" {
    const cmd = try parse("SET name zigkv");

    switch (cmd) {
        .set => |args| {
            try std.testing.expectEqualStrings("name", args.key);
            try std.testing.expectEqualStrings("zigkv", args.value);
        },
        else => return error.UnexpectedCommand,
    }
}

test "parse setex" {
    const cmd = try parse("SETEX tmp 1000 value");

    switch (cmd) {
        .setex => |args| {
            try std.testing.expectEqualStrings("tmp", args.key);
            try std.testing.expectEqual(@as(i64, 1000), args.ttl_ms);
            try std.testing.expectEqualStrings("value", args.value);
        },
        else => return error.UnexpectedCommand,
    }
}

test "reject unknown command" {
    try std.testing.expectError(ParseError.UnknownCommand, parse("HELLO"));
}

test "parse command case insensitive" {
    const cmd = try parse("get Name");

    switch (cmd) {
        .get => |key| try std.testing.expectEqualStrings("Name", key),
        else => return error.UnexpectedCommand,
    }
}

test "parse trims whitespace" {
    const cmd = try parse("  SET   name   zigkv  \r\n");

    switch (cmd) {
        .set => |args| {
            try std.testing.expectEqualStrings("name", args.key);
            try std.testing.expectEqualStrings("zigkv", args.value);
        },
        else => return error.UnexpectedCommand,
    }
}

test "reject empty command" {
    try std.testing.expectError(ParseError.EmptyCommand, parse(""));
    try std.testing.expectError(ParseError.EmptyCommand, parse("   \r\n"));
}

test "reject invalid arity" {
    try std.testing.expectError(ParseError.InvalidArity, parse("GET"));
    try std.testing.expectError(ParseError.InvalidArity, parse("GET a b"));
    try std.testing.expectError(ParseError.InvalidArity, parse("SET a"));
    try std.testing.expectError(ParseError.InvalidArity, parse("SET a b c"));
    try std.testing.expectError(ParseError.InvalidArity, parse("DEL"));
    try std.testing.expectError(ParseError.InvalidArity, parse("EXISTS"));
    try std.testing.expectError(ParseError.InvalidArity, parse("PING extra"));
}

test "reject invalid setex ttl" {
    try std.testing.expectError(ParseError.InvalidInteger, parse("SETEX tmp abc value"));
}

test "parse clear" {
    const cmd = try parse("CLEAR");
    try std.testing.expect(cmd == .clear);
}

test "parse ttl" {
    const cmd = try parse("TTL tmp");

    switch (cmd) {
        .ttl => |key| try std.testing.expectEqualStrings("tmp", key),
        else => return error.UnexpectedCommand,
    }
}

test "parse persist" {
    const cmd = try parse("PERSIST tmp");

    switch (cmd) {
        .persist => |key| try std.testing.expectEqualStrings("tmp", key),
        else => return error.UnexpectedCommand,
    }
}

test "parse keys" {
    const cmd = try parse("KEYS");
    try std.testing.expect(cmd == .keys);
}

test "parse dbsize" {
    const cmd = try parse("DBSIZE");
    try std.testing.expect(cmd == .dbsize);
}

test "parse all supported commands" {
    try std.testing.expect((try parse("PING")) == .ping);
    try std.testing.expect((try parse("CLEAR")) == .clear);
    try std.testing.expect((try parse("KEYS")) == .keys);
    try std.testing.expect((try parse("DBSIZE")) == .dbsize);

    switch (try parse("GET a")) {
        .get => |key| try std.testing.expectEqualStrings("a", key),
        else => return error.UnexpectedCommand,
    }

    switch (try parse("SET a 1")) {
        .set => |args| {
            try std.testing.expectEqualStrings("a", args.key);
            try std.testing.expectEqualStrings("1", args.value);
        },
        else => return error.UnexpectedCommand,
    }

    switch (try parse("DEL a")) {
        .del => |key| try std.testing.expectEqualStrings("a", key),
        else => return error.UnexpectedCommand,
    }

    switch (try parse("EXISTS a")) {
        .exists => |key| try std.testing.expectEqualStrings("a", key),
        else => return error.UnexpectedCommand,
    }

    switch (try parse("SETEX a 10 1")) {
        .setex => |args| {
            try std.testing.expectEqualStrings("a", args.key);
            try std.testing.expectEqual(@as(i64, 10), args.ttl_ms);
            try std.testing.expectEqualStrings("1", args.value);
        },
        else => return error.UnexpectedCommand,
    }

    switch (try parse("TTL a")) {
        .ttl => |key| try std.testing.expectEqualStrings("a", key),
        else => return error.UnexpectedCommand,
    }

    switch (try parse("PERSIST a")) {
        .persist => |key| try std.testing.expectEqualStrings("a", key),
        else => return error.UnexpectedCommand,
    }
}
