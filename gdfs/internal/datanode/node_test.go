package datanode

import (
	"context"
	"io"
	"strings"
	"testing"

	"github.com/stretchr/testify/require"
)

func TestDataNodePutGetBlock(t *testing.T) {
	store := NewLocalBlockStore(t.TempDir())

	node, err := NewDataNode("node-1", "127.0.0.1:9001", store)
	require.NoError(t, err)

	ctx := context.Background()

	info, err := node.PutBlock(ctx, &Block{
		ID:   BlockID("block-001"),
		Data: strings.NewReader("hello datanode"),
	})
	require.NoError(t, err)
	require.Equal(t, BlockID("block-001"), info.ID)

	block, err := node.GetBlock(ctx, BlockID("block-001"))
	require.NoError(t, err)

	rc, ok := block.Data.(io.ReadCloser)
	require.True(t, ok)
	defer rc.Close()

	data, err := io.ReadAll(block.Data)
	require.NoError(t, err)
	require.Equal(t, "hello datanode", string(data))
}

func TestNewDataNodeValidation(t *testing.T) {
	store := NewLocalBlockStore(t.TempDir())

	_, err := NewDataNode("", "127.0.0.1:9001", store)
	require.Error(t, err)

	_, err = NewDataNode("node-1", "127.0.0.1:9001", nil)
	require.Error(t, err)
}
