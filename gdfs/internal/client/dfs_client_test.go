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

func TestDFSClientPutGetStatDeleteFile(t *testing.T) {
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

	fileClient, err := NewDFSClient(
		5,
		1,
		dnServer.URL,
		func(addr string) BlockClient {
			return datanode.NewHTTPClient(addr)
		},
		metadataClient,
	)
	require.NoError(t, err)

	meta, err := fileClient.PutFile(ctx, "/docs/hello.txt", strings.NewReader("hello-world"))
	require.NoError(t, err)

	require.Equal(t, namenode.FilePath("/docs/hello.txt"), meta.Path)
	require.Equal(t, int64(len("hello-world")), meta.Size)
	require.Len(t, meta.Blocks, 3)

	stat, err := fileClient.StatFile(ctx, "/docs/hello.txt")
	require.NoError(t, err)
	require.Equal(t, meta, stat)

	var out bytes.Buffer

	n, err := fileClient.GetFile(ctx, "/docs/hello.txt", &out)
	require.NoError(t, err)

	require.Equal(t, int64(len("hello-world")), n)
	require.Equal(t, "hello-world", out.String())

	err = fileClient.DeleteFile(ctx, "/docs/hello.txt")
	require.NoError(t, err)

	_, err = fileClient.StatFile(ctx, "/docs/hello.txt")
	require.Error(t, err)
}
