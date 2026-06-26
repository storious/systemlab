package datanode

import (
	"context"
	"io"
	"strings"
	"testing"

	"github.com/stretchr/testify/require"
)

func TestLocalBlockStorePutGet(t *testing.T) {
	store := NewLocalBlockStore(t.TempDir())
	ctx := context.Background()

	info, err := store.Put(ctx, &Block{
		ID:   BlockID("block-001"),
		Data: strings.NewReader("hello block"),
	})
	require.NoError(t, err)

	require.Equal(t, BlockID("block-001"), info.ID)
	require.Equal(t, int64(len("hello block")), info.Size)
	require.NotEmpty(t, info.Checksum)

	block, err := store.Get(ctx, BlockID("block-001"))
	require.NoError(t, err)

	rc, ok := block.Data.(io.ReadCloser)
	require.True(t, ok)
	defer rc.Close()

	data, err := io.ReadAll(block.Data)
	require.NoError(t, err)

	require.Equal(t, "hello block", string(data))
}

func TestLocalBlockStoreExistsDelete(t *testing.T) {
	store := NewLocalBlockStore(t.TempDir())
	ctx := context.Background()

	id := BlockID("block-001")

	require.False(t, store.Exists(ctx, id))

	_, err := store.Put(ctx, &Block{
		ID:   id,
		Data: strings.NewReader("hello"),
	})
	require.NoError(t, err)

	require.True(t, store.Exists(ctx, id))

	err = store.Delete(ctx, id)
	require.NoError(t, err)

	require.False(t, store.Exists(ctx, id))
}

func TestLocalBlockStoreStat(t *testing.T) {
	store := NewLocalBlockStore(t.TempDir())
	ctx := context.Background()

	id := BlockID("block-001")

	written, err := store.Put(ctx, &Block{
		ID:   id,
		Data: strings.NewReader("hello"),
	})
	require.NoError(t, err)

	stat, err := store.Stat(ctx, id)
	require.NoError(t, err)

	require.Equal(t, written.ID, stat.ID)
	require.Equal(t, written.Size, stat.Size)
	require.Equal(t, written.Checksum, stat.Checksum)
}
