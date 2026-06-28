package client

import (
	"bytes"
	"context"
	"net/http/httptest"
	"strings"
	"testing"

	"gdfs/internal/datanode"

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

// compile-time checks
var (
	_ BlockWriter = (*datanode.HTTPClient)(nil)
	_ BlockReader = (*datanode.HTTPClient)(nil)
)
