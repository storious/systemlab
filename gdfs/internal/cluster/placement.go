package cluster

import (
	"sort"
)

type PlacementPolicy interface {
	Allocate(blockSize uint64, replicas int, nodes []DataNodeInfo) ([]DataNodeInfo, error)
}

type LeastUsedPlacement struct{}

func NewLeastUsedPlacement() *LeastUsedPlacement {
	return &LeastUsedPlacement{}
}

func (p *LeastUsedPlacement) Allocate(blockSize uint64, replicas int, nodes []DataNodeInfo) ([]DataNodeInfo, error) {
	if replicas <= 0 {
		return nil, ErrNotEnoughDataNodes
	}

	candidates := make([]DataNodeInfo, 0, len(nodes))
	for _, node := range nodes {
		if node.State != NodeAlive {
			continue
		}
		if node.Capacity < node.Used {
			continue
		}
		if node.Capacity-node.Used < blockSize {
			continue
		}
		candidates = append(candidates, node)
	}

	if len(candidates) == 0 {
		return nil, ErrNoAliveDataNodes
	}

	if len(candidates) < replicas {
		return nil, ErrNotEnoughDataNodes
	}

	sort.Slice(candidates, func(i, j int) bool {
		leftFree := candidates[i].Capacity - candidates[i].Used
		rightFree := candidates[j].Capacity - candidates[j].Used
		return leftFree > rightFree
	})

	return candidates[:replicas], nil
}
