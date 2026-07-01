const std = @import("std");

pub fn ok(allocator: std.mem.Allocator) ![]u8 {
    return allocator.dupe(u8, "+OK\r\n");
}

pub fn pong(allocator: std.mem.Allocator) ![]u8 {
    return allocator.dupe(u8, "+PONG\r\n");
}

pub fn nil(allocator: std.mem.Allocator) ![]u8 {
    return allocator.dupe(u8, "$nil\r\n");
}

pub fn bulk(allocator: std.mem.Allocator, value: []const u8) ![]u8 {
    return std.fmt.allocPrint(allocator, "${s}\r\n", .{value});
}

pub fn integer(allocator: std.mem.Allocator, value: bool) ![]u8 {
    return std.fmt.allocPrint(allocator, ":{d}\r\n", .{@intFromBool(value)});
}

pub fn err(allocator: std.mem.Allocator, name: []const u8) ![]u8 {
    return std.fmt.allocPrint(allocator, "-ERR {s}\r\n", .{name});
}

pub fn integerValue(allocator: std.mem.Allocator, value: i64) ![]u8 {
    return std.fmt.allocPrint(allocator, ":{d}\r\n", .{value});
}

pub fn list(allocator: std.mem.Allocator, items: []const []const u8) ![]u8 {
    var out = std.ArrayList(u8){
        .items = &.{},
        .capacity = 0,
    };
    defer out.deinit(allocator);

    try out.append(allocator, '$');

    for (items, 0..) |item, i| {
        if (i > 0) try out.append(allocator, ' ');
        try out.appendSlice(allocator, item);
    }

    try out.appendSlice(allocator, "\r\n");
    return out.toOwnedSlice(allocator);
}

test "format list response" {
    const items = [_][]const u8{ "a", "b" };
    const resp = try list(std.testing.allocator, &items);
    defer std.testing.allocator.free(resp);

    try std.testing.expectEqualStrings("$a b\r\n", resp);
}

test "format ok response" {
    const resp = try ok(std.testing.allocator);
    defer std.testing.allocator.free(resp);

    try std.testing.expectEqualStrings("+OK\r\n", resp);
}

test "format pong response" {
    const resp = try pong(std.testing.allocator);
    defer std.testing.allocator.free(resp);

    try std.testing.expectEqualStrings("+PONG\r\n", resp);
}

test "format nil response" {
    const resp = try nil(std.testing.allocator);
    defer std.testing.allocator.free(resp);

    try std.testing.expectEqualStrings("$nil\r\n", resp);
}

test "format bulk response" {
    const resp = try bulk(std.testing.allocator, "zigkv");
    defer std.testing.allocator.free(resp);

    try std.testing.expectEqualStrings("$zigkv\r\n", resp);
}

test "format integer bool response" {
    const yes = try integer(std.testing.allocator, true);
    defer std.testing.allocator.free(yes);

    const no = try integer(std.testing.allocator, false);
    defer std.testing.allocator.free(no);

    try std.testing.expectEqualStrings(":1\r\n", yes);
    try std.testing.expectEqualStrings(":0\r\n", no);
}

test "format integer value response" {
    const resp = try integerValue(std.testing.allocator, -2);
    defer std.testing.allocator.free(resp);

    try std.testing.expectEqualStrings(":-2\r\n", resp);
}

test "format error response" {
    const resp = try err(std.testing.allocator, "UnknownCommand");
    defer std.testing.allocator.free(resp);

    try std.testing.expectEqualStrings("-ERR UnknownCommand\r\n", resp);
}
