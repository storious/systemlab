package datanode

import "context"

type BlockStore interface {
	Put(ctx context.Context, block *Block) (BlockInfo, error)
	Get(ctx context.Context, id BlockID) (*Block, error)
	Delete(ctx context.Context, id BlockID) error
	Exists(ctx context.Context, id BlockID) bool
	Stat(ctx context.Context, id BlockID) (BlockInfo, error)
}
