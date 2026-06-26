package datanode

import "io"

type BlockID string

type Block struct {
	ID   BlockID
	Data io.Reader
}

type BlockInfo struct {
	ID       BlockID
	Size     int64
	Checksum string
}
