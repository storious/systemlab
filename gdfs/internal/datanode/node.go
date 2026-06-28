package datanode

import (
	"context"
	"errors"
)

type NodeID string

type DataNode struct {
	ID    NodeID
	Addr  string
	Store BlockStore
}

func NewDataNode(id NodeID, addr string, store BlockStore) (*DataNode, error) {
	if id == "" {
		return nil, errors.New("empty datanode id")
	}
	if store == nil {
		return nil, errors.New("nil block store")
	}

	return &DataNode{
		ID:    id,
		Addr:  addr,
		Store: store,
	}, nil
}

func (n *DataNode) PutBlock(ctx context.Context, block *Block) (BlockInfo, error) {
	return n.Store.Put(ctx, block)
}

func (n *DataNode) GetBlock(ctx context.Context, id BlockID) (*Block, error) {
	return n.Store.Get(ctx, id)
}

func (n *DataNode) DeleteBlock(ctx context.Context, id BlockID) error {
	return n.Store.Delete(ctx, id)
}

func (n *DataNode) ExistsBlock(ctx context.Context, id BlockID) bool {
	return n.Store.Exists(ctx, id)
}

func (n *DataNode) StatBlock(ctx context.Context, id BlockID) (BlockInfo, error) {
	return n.Store.Stat(ctx, id)
}
