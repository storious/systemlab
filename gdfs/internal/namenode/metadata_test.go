package namenode

import (
	"testing"

	"gdfs/internal/datanode"

	"github.com/stretchr/testify/require"
)

func TestMetadataStorePutGetFile(t *testing.T) {
	store := NewMetadataStore()

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

	err := store.PutFile(meta)
	require.NoError(t, err)

	got, err := store.GetFile("/docs/hello.txt")
	require.NoError(t, err)

	require.Equal(t, meta.Path, got.Path)
	require.Equal(t, meta.Size, got.Size)
	require.Equal(t, meta.Blocks, got.Blocks)
}

func TestMetadataStoreDeleteFile(t *testing.T) {
	store := NewMetadataStore()

	err := store.PutFile(FileMetadata{
		Path: "/docs/hello.txt",
		Size: 5,
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
	})
	require.NoError(t, err)

	require.True(t, store.Exists("/docs/hello.txt"))

	err = store.DeleteFile("/docs/hello.txt")
	require.NoError(t, err)

	require.False(t, store.Exists("/docs/hello.txt"))

	_, err = store.GetFile("/docs/hello.txt")
	require.Error(t, err)
}

func TestMetadataStoreRejectsEmptyPath(t *testing.T) {
	store := NewMetadataStore()

	err := store.PutFile(FileMetadata{})
	require.Error(t, err)

	_, err = store.GetFile("")
	require.Error(t, err)

	err = store.DeleteFile("")
	require.Error(t, err)
}
