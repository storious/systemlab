const std = @import("std");
const command = @import("command.zig");
const response = @import("response.zig");

pub const ProtocolError = command.ParseError;

pub fn parse(input: []const u8) ProtocolError!command.Command {
    return command.parse(input);
}

pub fn formatError(allocator: std.mem.Allocator, err: anyerror) ![]u8 {
    return response.err(allocator, @errorName(err));
}
