package namenode

import (
	"context"
	"net/http/httptest"
	"testing"

	"gdfs/internal/cluster"
	"gdfs/internal/datanode"

	"github.com/stretchr/testify/require"
)

func TestHTTPClientPutGetDeleteFile(t *testing.T) {
	node, err := NewNameNode(NewMetadataStore())
	require.NoError(t, err)

	server := httptest.NewServer(NewHTTPServer(node))
	defer server.Close()

	client := NewHTTPClient(server.URL)
	ctx := context.Background()

	meta := FileMetadata{
		Path: "/docs/hello.txt",
		Size: 11,
		Blocks: []BlockMetadata{
			{
				Info: datanode.BlockInfo{
					ID:       "block-001",
					Size:     5,
					Checksum: "a",
				},
				Replicas: []BlockReplica{
					{NodeID: "node-1", Addr: "http://localhost:9001"},
				},
			},
		},
	}

	created, err := client.PutFile(ctx, meta)
	require.NoError(t, err)
	require.Equal(t, meta, created)

	got, err := client.GetFile(ctx, "/docs/hello.txt")
	require.NoError(t, err)
	require.Equal(t, meta, got)

	err = client.DeleteFile(ctx, "/docs/hello.txt")
	require.NoError(t, err)

	_, err = client.GetFile(ctx, "/docs/hello.txt")
	require.Error(t, err)
}

func TestHTTPClientAllocateBlock(t *testing.T) {
	node, err := NewNameNode(NewMetadataStore())
	require.NoError(t, err)

	ctx := context.Background()

	require.NoError(t, node.RegisterDataNode(ctx, cluster.DataNodeInfo{
		ID:       "node-1",
		Addr:     "http://localhost:9001",
		Capacity: 1000,
		Used:     900,
	}))

	require.NoError(t, node.RegisterDataNode(ctx, cluster.DataNodeInfo{
		ID:       "node-2",
		Addr:     "http://localhost:9002",
		Capacity: 1000,
		Used:     100,
	}))

	server := httptest.NewServer(NewHTTPServer(node))
	defer server.Close()

	client := NewHTTPClient(server.URL)

	nodes, err := client.AllocateBlock(ctx, 100, 1)
	require.NoError(t, err)
	require.Len(t, nodes, 1)
	require.Equal(t, cluster.DataNodeID("node-2"), nodes[0].ID)
}
