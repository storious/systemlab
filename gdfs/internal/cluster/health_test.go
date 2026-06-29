package cluster

import (
	"testing"
	"time"

	"github.com/stretchr/testify/require"
)

func TestRegistryEvaluateHealth(t *testing.T) {
	registry := NewRegistry()

	now := time.Now()
	cfg := HealthConfig{
		SuspectAfter: 10 * time.Second,
		DeadAfter:    30 * time.Second,
	}

	require.NoError(t, registry.Register(DataNodeInfo{
		ID:       "alive",
		Addr:     "http://localhost:9001",
		LastSeen: now.Add(-5 * time.Second),
	}))

	require.NoError(t, registry.Register(DataNodeInfo{
		ID:       "suspect",
		Addr:     "http://localhost:9002",
		LastSeen: now.Add(-15 * time.Second),
	}))

	require.NoError(t, registry.Register(DataNodeInfo{
		ID:       "dead",
		Addr:     "http://localhost:9003",
		LastSeen: now.Add(-45 * time.Second),
	}))

	registry.EvaluateHealth(now, cfg)

	alive, ok := registry.Get("alive")
	require.True(t, ok)
	require.Equal(t, NodeAlive, alive.State)

	suspect, ok := registry.Get("suspect")
	require.True(t, ok)
	require.Equal(t, NodeSuspect, suspect.State)

	dead, ok := registry.Get("dead")
	require.True(t, ok)
	require.Equal(t, NodeDead, dead.State)

	aliveNodes := registry.AliveNodes()
	require.Len(t, aliveNodes, 1)
	require.Equal(t, DataNodeID("alive"), aliveNodes[0].ID)
}
