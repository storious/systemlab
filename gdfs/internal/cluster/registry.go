package cluster

import (
	"errors"
	"sync"
	"time"
)

type DataNodeID string

type NodeState int

const (
	NodeUnknown NodeState = iota
	NodeAlive
	NodeSuspect
	NodeDead
)

type DataNodeInfo struct {
	ID       DataNodeID
	Addr     string
	Capacity uint64
	Used     uint64
	LastSeen time.Time
	State    NodeState
}

type Registry struct {
	mu    sync.RWMutex
	nodes map[DataNodeID]DataNodeInfo
}

func NewRegistry() *Registry {
	return &Registry{
		nodes: make(map[DataNodeID]DataNodeInfo),
	}
}

func (r *Registry) Register(info DataNodeInfo) error {
	if info.ID == "" {
		return errors.New("empty datanode id")
	}
	if info.Addr == "" {
		return errors.New("empty datanode address")
	}

	r.mu.Lock()
	defer r.mu.Unlock()

	if info.LastSeen.IsZero() {
		info.LastSeen = time.Now()
	}

	if info.State == NodeUnknown {
		info.State = NodeAlive
	}

	r.nodes[info.ID] = info
	return nil
}

func (r *Registry) AliveNodes() []DataNodeInfo {
	r.mu.RLock()
	defer r.mu.RUnlock()

	out := make([]DataNodeInfo, 0)
	for _, info := range r.nodes {
		if info.State == NodeAlive {
			out = append(out, info)
		}
	}
	return out
}

func (r *Registry) UpdateState(id DataNodeID, state NodeState) bool {
	r.mu.Lock()
	defer r.mu.Unlock()

	info, ok := r.nodes[id]
	if !ok {
		return false
	}

	info.State = state
	r.nodes[id] = info
	return true
}

func (r *Registry) Get(id DataNodeID) (DataNodeInfo, bool) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	info, ok := r.nodes[id]
	return info, ok
}

func (r *Registry) List() []DataNodeInfo {
	r.mu.RLock()
	defer r.mu.RUnlock()

	out := make([]DataNodeInfo, 0, len(r.nodes))
	for _, info := range r.nodes {
		out = append(out, info)
	}

	return out
}

func (r *Registry) Unregister(id DataNodeID) {
	r.mu.Lock()
	defer r.mu.Unlock()

	delete(r.nodes, id)
}
