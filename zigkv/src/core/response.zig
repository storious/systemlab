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
