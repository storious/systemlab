package client

import (
	"bytes"
	"context"
	"net/http/httptest"
	"strings"
	"testing"

	"gdfs/internal/cluster"
	"gdfs/internal/datanode"
	"gdfs/internal/namenode"

	"github.com/stretchr/testify/require"
)

func TestWriterReaderWithHTTPDataNode(t *testing.T) {
	ctx := context.Background()

	store := datanode.NewLocalBlockStore(t.TempDir())

	node, err := datanode.NewDataNode("node-1", "127.0.0.1:0", store)
	require.NoError(t, err)

	server := httptest.NewServer(datanode.NewHTTPServer(node))
	defer server.Close()

	dnClient := datanode.NewHTTPClient(server.URL)

	writer, err := NewWriter(5, dnClient)
	require.NoError(t, err)

	input := "hello-world-from-gdfs"

	result, err := writer.Write(ctx, strings.NewReader(input))
	require.NoError(t, err)
	require.Equal(t, int64(len(input)), result.Size)
	require.NotEmpty(t, result.Blocks)

	reader, err := NewReader(dnClient)
	require.NoError(t, err)

	var out bytes.Buffer

	n, err := reader.Read(ctx, result.Blocks, &out)
	require.NoError(t, err)

	require.Equal(t, int64(len(input)), n)
	require.Equal(t, input, out.String())
}

func TestWriterReaderWithEmptyInput(t *testing.T) {
	ctx := context.Background()

	store := datanode.NewLocalBlockStore(t.TempDir())

	node, err := datanode.NewDataNode("node-1", "127.0.0.1:0", store)
	require.NoError(t, err)

	server := httptest.NewServer(datanode.NewHTTPServer(node))
	defer server.Close()

	dnClient := datanode.NewHTTPClient(server.URL)

	writer, err := NewWriter(5, dnClient)
	require.NoError(t, err)

	result, err := writer.Write(ctx, strings.NewReader(""))
	require.NoError(t, err)

	require.Equal(t, int64(0), result.Size)
	require.Empty(t, result.Blocks)

	reader, err := NewReader(dnClient)
	require.NoError(t, err)

	var out bytes.Buffer

	n, err := reader.Read(ctx, result.Blocks, &out)
	require.NoError(t, err)

	require.Equal(t, int64(0), n)
	require.Equal(t, "", out.String())
}
func TestSingleNodeDFSClientEndToEnd(t *testing.T) {
	ctx := context.Background()

	dnStore := datanode.NewLocalBlockStore(t.TempDir())
	dn, err := datanode.NewDataNode("node-1", "127.0.0.1:0", dnStore)
	require.NoError(t, err)

	dnServer := httptest.NewServer(datanode.NewHTTPServer(dn))
	defer dnServer.Close()

	nn, err := namenode.NewNameNode(namenode.NewMetadataStore())
	require.NoError(t, err)

	require.NoError(t, nn.RegisterDataNode(ctx, cluster.DataNodeInfo{
		ID:       "node-1",
		Addr:     dnServer.URL,
		Capacity: 1024 * 1024,
		Used:     0,
	}))

	nnServer := httptest.NewServer(namenode.NewHTTPServer(nn))
	defer nnServer.Close()

	metadataClient := namenode.NewHTTPClient(nnServer.URL)

	dfs, err := NewDFSClient(
		5,
		1,
		func(addr string) BlockClient {
			return datanode.NewHTTPClient(addr)
		},
		metadataClient,
	)
	require.NoError(t, err)

	input := "hello-world-from-gdfs"

	meta, err := dfs.PutFile(ctx, "/docs/hello.txt", strings.NewReader(input))
	require.NoError(t, err)
	require.Equal(t, namenode.FilePath("/docs/hello.txt"), meta.Path)
	require.Equal(t, int64(len(input)), meta.Size)
	require.Len(t, meta.Blocks, 5)

	stat, err := dfs.StatFile(ctx, "/docs/hello.txt")
	require.NoError(t, err)
	require.Equal(t, meta, stat)

	var out bytes.Buffer

	n, err := dfs.GetFile(ctx, "/docs/hello.txt", &out)
	require.NoError(t, err)
	require.Equal(t, int64(len(input)), n)
	require.Equal(t, input, out.String())

	err = dfs.DeleteFile(ctx, "/docs/hello.txt")
	require.NoError(t, err)

	_, err = dfs.StatFile(ctx, "/docs/hello.txt")
	require.Error(t, err)
}

func TestDFSClientWritesTwoReplicasAndReadsFromFallbackReplica(t *testing.T) {
	ctx := context.Background()

	dnStore1 := datanode.NewLocalBlockStore(t.TempDir())
	dn1, err := datanode.NewDataNode("node-1", "127.0.0.1:0", dnStore1)
	require.NoError(t, err)
	dnServer1 := httptest.NewServer(datanode.NewHTTPServer(dn1))
	defer dnServer1.Close()

	dnStore2 := datanode.NewLocalBlockStore(t.TempDir())
	dn2, err := datanode.NewDataNode("node-2", "127.0.0.1:0", dnStore2)
	require.NoError(t, err)
	dnServer2 := httptest.NewServer(datanode.NewHTTPServer(dn2))
	defer dnServer2.Close()

	nn, err := namenode.NewNameNode(namenode.NewMetadataStore())
	require.NoError(t, err)

	require.NoError(t, nn.RegisterDataNode(ctx, cluster.DataNodeInfo{
		ID:       "node-1",
		Addr:     dnServer1.URL,
		Capacity: 1024 * 1024,
		Used:     0,
	}))
	require.NoError(t, nn.RegisterDataNode(ctx, cluster.DataNodeInfo{
		ID:       "node-2",
		Addr:     dnServer2.URL,
		Capacity: 1024 * 1024,
		Used:     0,
	}))

	nnServer := httptest.NewServer(namenode.NewHTTPServer(nn))
	defer nnServer.Close()

	metadataClient := namenode.NewHTTPClient(nnServer.URL)

	dfs, err := NewDFSClient(
		5,
		2,
		func(addr string) BlockClient {
			return datanode.NewHTTPClient(addr)
		},
		metadataClient,
	)
	require.NoError(t, err)

	input := "hello-world-from-replicated-gdfs"

	meta, err := dfs.PutFile(ctx, "/docs/replicated.txt", strings.NewReader(input))
	require.NoError(t, err)
	require.NotEmpty(t, meta.Blocks)

	for _, block := range meta.Blocks {
		require.Len(t, block.Replicas, 2)
	}

	for _, block := range meta.Blocks {
		require.NoError(t, dnStore1.Delete(ctx, block.Info.ID))
	}

	var out bytes.Buffer

	n, err := dfs.GetFile(ctx, "/docs/replicated.txt", &out)
	require.NoError(t, err)

	require.Equal(t, int64(len(input)), n)
	require.Equal(t, input, out.String())
}

func TestDFSClientReplicatedWriteUpdatesDataNodeUsage(t *testing.T) {
	ctx := context.Background()

	dnStore1 := datanode.NewLocalBlockStore(t.TempDir())
	dn1, err := datanode.NewDataNode("node-1", "127.0.0.1:0", dnStore1)
	require.NoError(t, err)
	dnServer1 := httptest.NewServer(datanode.NewHTTPServer(dn1))
	defer dnServer1.Close()

	dnStore2 := datanode.NewLocalBlockStore(t.TempDir())
	dn2, err := datanode.NewDataNode("node-2", "127.0.0.1:0", dnStore2)
	require.NoError(t, err)
	dnServer2 := httptest.NewServer(datanode.NewHTTPServer(dn2))
	defer dnServer2.Close()

	nn, err := namenode.NewNameNode(namenode.NewMetadataStore())
	require.NoError(t, err)

	require.NoError(t, nn.Heartbeat(ctx, cluster.Heartbeat{
		ID:       "node-1",
		Addr:     dnServer1.URL,
		Capacity: 1024 * 1024,
		Used:     0,
	}))
	require.NoError(t, nn.Heartbeat(ctx, cluster.Heartbeat{
		ID:       "node-2",
		Addr:     dnServer2.URL,
		Capacity: 1024 * 1024,
		Used:     0,
	}))

	nnServer := httptest.NewServer(namenode.NewHTTPServer(nn))
	defer nnServer.Close()

	metadataClient := namenode.NewHTTPClient(nnServer.URL)

	dfs, err := NewDFSClient(
		5,
		2,
		func(addr string) BlockClient {
			return datanode.NewHTTPClient(addr)
		},
		metadataClient,
	)
	require.NoError(t, err)

	input := "hello-replicated-usage"

	meta, err := dfs.PutFile(ctx, "/docs/usage.txt", strings.NewReader(input))
	require.NoError(t, err)
	require.NotEmpty(t, meta.Blocks)

	for _, block := range meta.Blocks {
		require.Len(t, block.Replicas, 2)
	}

	stats1, err := dn1.Stats()
	require.NoError(t, err)
	require.Equal(t, uint64(len(input)), stats1.Used)

	stats2, err := dn2.Stats()
	require.NoError(t, err)
	require.Equal(t, uint64(len(input)), stats2.Used)

	require.NoError(t, metadataClient.Heartbeat(ctx, cluster.Heartbeat{
		ID:       "node-1",
		Addr:     dnServer1.URL,
		Capacity: stats1.Capacity,
		Used:     stats1.Used,
	}))
	require.NoError(t, metadataClient.Heartbeat(ctx, cluster.Heartbeat{
		ID:       "node-2",
		Addr:     dnServer2.URL,
		Capacity: stats2.Capacity,
		Used:     stats2.Used,
	}))

	nodes := nn.ListDataNodes(ctx)
	require.Len(t, nodes, 2)

	for _, node := range nodes {
		require.Equal(t, uint64(len(input)), node.Used)
	}
}
