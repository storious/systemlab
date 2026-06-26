package datanode

import (
	"io"
	"strings"
	"testing"

	"github.com/stretchr/testify/require"
)

func TestWriteAndReadBlock(t *testing.T) {
	store := NewLocalBlockStore(t.TempDir())

	input := "hello block"

	n, err := store.WriteBlock("block-001", strings.NewReader(input))
	require.NoError(t, err)
	require.Equal(t, int64(len(input)), n)

	size, r, err := store.ReadBlock("block-001")
	require.NoError(t, err)
	defer r.Close()

	b, err := io.ReadAll(r)
	require.NoError(t, err)

	require.Equal(t, int64(len(input)), size)
	require.Equal(t, input, string(b))
}

func TestHasAndDeleteBlock(t *testing.T) {
	store := NewLocalBlockStore(t.TempDir())

	require.False(t, store.HasBlock("block-001"))

	_, err := store.WriteBlock("block-001", strings.NewReader("hello"))
	require.NoError(t, err)

	require.True(t, store.HasBlock("block-001"))

	err = store.DeleteBlock("block-001")
	require.NoError(t, err)

	require.False(t, store.HasBlock("block-001"))
}
