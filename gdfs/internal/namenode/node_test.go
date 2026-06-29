package namenode

import (
	"context"
	"testing"

	"gdfs/internal/datanode"

	"github.com/stretchr/testify/require"
)

func TestNameNodeCreateGetDeleteFile(t *testing.T) {
	node, err := NewNameNode(NewMetadataStore())
	require.NoError(t, err)

	ctx := context.Background()

	meta := FileMetadata{
		Path: "/docs/hello.txt",
		Size: 11,
		Blocks: []datanode.BlockInfo{
			{ID: "block-001", Size: 5, Checksum: "a"},
			{ID: "block-002", Size: 6, Checksum: "b"},
		},
	}

	err = node.CreateFile(ctx, meta)
	require.NoError(t, err)

	require.True(t, node.ExistsFile(ctx, "/docs/hello.txt"))

	got, err := node.GetFile(ctx, "/docs/hello.txt")
	require.NoError(t, err)
	require.Equal(t, meta, got)

	err = node.DeleteFile(ctx, "/docs/hello.txt")
	require.NoError(t, err)

	require.False(t, node.ExistsFile(ctx, "/docs/hello.txt"))
}

func TestNewNameNodeRejectsNilStore(t *testing.T) {
	node, err := NewNameNode(nil)

	require.Error(t, err)
	require.Nil(t, node)
}

func TestNameNodeRegisterListUnregisterDataNode(t *testing.T) {
	node, err := NewNameNode(NewMetadataStore())
	require.NoError(t, err)

	ctx := context.Background()

	err = node.RegisterDataNode(ctx, DataNodeInfo{
		ID:       "node-1",
		Addr:     "http://localhost:9001",
		Capacity: 1024,
		Used:     128,
	})
	require.NoError(t, err)

	nodes := node.ListDataNodes(ctx)
	require.Len(t, nodes, 1)
	require.Equal(t, DataNodeID("node-1"), nodes[0].ID)

	got, ok := node.GetDataNode(ctx, "node-1")
	require.True(t, ok)
	require.Equal(t, "http://localhost:9001", got.Addr)

	node.UnregisterDataNode(ctx, "node-1")

	_, ok = node.GetDataNode(ctx, "node-1")
	require.False(t, ok)
}
