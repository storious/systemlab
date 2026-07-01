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

fn emptyByteList() std.ArrayList(u8) {
    return .{
        .items = &.{},
        .capacity = 0,
    };
}

pub fn list(allocator: std.mem.Allocator, items: []const []const u8) ![]u8 {
    var out = emptyByteList();
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
