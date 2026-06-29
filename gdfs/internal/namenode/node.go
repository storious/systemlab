package namenode

import (
	"context"
	"errors"
	"time"

	"gdfs/internal/cluster"
)

type NameNode struct {
	store     *MetadataStore
	registry  *cluster.Registry
	placement cluster.PlacementPolicy
}

func NewNameNode(store *MetadataStore) (*NameNode, error) {
	if store == nil {
		return nil, errors.New("nil metadata store")
	}

	return &NameNode{
		store:     store,
		registry:  cluster.NewRegistry(),
		placement: cluster.NewLeastUsedPlacement(),
	}, nil
}

func (n *NameNode) CreateFile(ctx context.Context, meta FileMetadata) error {
	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
	}

	return n.store.PutFile(meta)
}

func (n *NameNode) GetFile(ctx context.Context, path FilePath) (FileMetadata, error) {
	select {
	case <-ctx.Done():
		return FileMetadata{}, ctx.Err()
	default:
	}

	return n.store.GetFile(path)
}

func (n *NameNode) DeleteFile(ctx context.Context, path FilePath) error {
	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
	}

	return n.store.DeleteFile(path)
}

func (n *NameNode) ExistsFile(ctx context.Context, path FilePath) bool {
	select {
	case <-ctx.Done():
		return false
	default:
	}

	return n.store.Exists(path)
}

func (n *NameNode) RegisterDataNode(ctx context.Context, info cluster.DataNodeInfo) error {
	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
	}

	return n.registry.Register(info)
}

func (n *NameNode) GetDataNode(ctx context.Context, id cluster.DataNodeID) (cluster.DataNodeInfo, bool) {
	select {
	case <-ctx.Done():
		return cluster.DataNodeInfo{}, false
	default:
	}

	return n.registry.Get(id)
}

func (n *NameNode) ListDataNodes(ctx context.Context) []cluster.DataNodeInfo {
	select {
	case <-ctx.Done():
		return nil
	default:
	}

	return n.registry.List()
}

func (n *NameNode) UnregisterDataNode(ctx context.Context, id cluster.DataNodeID) {
	select {
	case <-ctx.Done():
		return
	default:
	}

	n.registry.Unregister(id)
}

func (n *NameNode) AliveDataNodes(ctx context.Context) []cluster.DataNodeInfo {
	select {
	case <-ctx.Done():
		return nil
	default:
	}

	return n.registry.AliveNodes()
}

func (n *NameNode) Heartbeat(ctx context.Context, hb cluster.Heartbeat) error {
	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
	}

	return n.registry.Heartbeat(hb)
}

func (n *NameNode) EvaluateClusterHealth(ctx context.Context, now time.Time, cfg cluster.HealthConfig) error {
	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
	}

	n.registry.EvaluateHealth(now, cfg)
	return nil
}

func (n *NameNode) AllocateBlock(ctx context.Context, blockSize uint64, replicas int) ([]cluster.DataNodeInfo, error) {
	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	default:
	}

	nodes := n.registry.AliveNodes()
	return n.placement.Allocate(blockSize, replicas, nodes)
}
