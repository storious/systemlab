package client

import (
	"bytes"
	"context"
	"net/http/httptest"
	"strings"
	"testing"

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

	nnServer := httptest.NewServer(namenode.NewHTTPServer(nn))
	defer nnServer.Close()

	blockClient := datanode.NewHTTPClient(dnServer.URL)
	metadataClient := namenode.NewHTTPClient(nnServer.URL)

	dfs, err := NewDFSClient(5, blockClient, metadataClient)
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
