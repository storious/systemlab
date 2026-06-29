package cluster

import "errors"

var (
	ErrEmptyDataNodeID      = errors.New("empty datanode id")
	ErrEmptyDataNodeAddr    = errors.New("empty datanode address")
	ErrNoAliveDataNodes     = errors.New("no alive datanodes")
	ErrNotEnoughDataNodes   = errors.New("not enough datanodes")
	ErrInsufficientCapacity = errors.New("insufficient datanode capacity")
)
