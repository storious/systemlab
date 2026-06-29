package namenode

import (
	"testing"

	"github.com/stretchr/testify/require"
)

func TestRegistryRegisterGetListUnregister(t *testing.T) {
	registry := NewRegistry()

	info := DataNodeInfo{
		ID:       "node-1",
		Addr:     "http://localhost:9001",
		Capacity: 1024,
		Used:     128,
	}

	err := registry.Register(info)
	require.NoError(t, err)

	got, ok := registry.Get("node-1")
	require.True(t, ok)
	require.Equal(t, info.ID, got.ID)
	require.Equal(t, info.Addr, got.Addr)
	require.Equal(t, info.Capacity, got.Capacity)
	require.Equal(t, info.Used, got.Used)
	require.False(t, got.LastSeen.IsZero())

	nodes := registry.List()
	require.Len(t, nodes, 1)

	registry.Unregister("node-1")

	_, ok = registry.Get("node-1")
	require.False(t, ok)
	require.Empty(t, registry.List())
}

func TestRegistryRejectsInvalidNode(t *testing.T) {
	registry := NewRegistry()

	err := registry.Register(DataNodeInfo{
		Addr: "http://localhost:9001",
	})
	require.Error(t, err)

	err = registry.Register(DataNodeInfo{
		ID: "node-1",
	})
	require.Error(t, err)
}
