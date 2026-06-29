package client

import (
	"bytes"
	"context"
	"io"

	"gdfs/internal/cluster"
	"gdfs/internal/datanode"
	"gdfs/internal/namenode"
)

type MetadataClient interface {
	AllocateBlock(ctx context.Context, blockSize uint64, replicas int) ([]cluster.DataNodeInfo, error)

	PutFile(ctx context.Context, meta namenode.FileMetadata) (namenode.FileMetadata, error)
	GetFile(ctx context.Context, path namenode.FilePath) (namenode.FileMetadata, error)
	DeleteFile(ctx context.Context, path namenode.FilePath) error
}
type BlockClientFactory func(addr string) BlockClient

type DFSClient struct {
	blockSize int64
	replicas  int

	defaultBlockAddr string
	blocks           BlockClientFactory
	metadata         MetadataClient
}

type BlockClient interface {
	BlockWriter
	BlockReader
}

func NewDFSClient(blockSize int64, replicas int, defaultBlockAddr string, blocks BlockClientFactory, metadata MetadataClient) (*DFSClient, error) {
	if blockSize <= 0 {
		return nil, ErrInvalidBlockSize
	}
	if replicas <= 0 {
		return nil, ErrInvalidReplicaCount
	}
	if blocks == nil {
		return nil, ErrNilBlockClientFactory
	}
	if metadata == nil {
		return nil, ErrNilMetadataClient
	}

	return &DFSClient{
		blockSize:        blockSize,
		replicas:         replicas,
		blocks:           blocks,
		metadata:         metadata,
		defaultBlockAddr: defaultBlockAddr,
	}, nil
}

func (c *DFSClient) PutFile(ctx context.Context, path namenode.FilePath, r io.Reader) (namenode.FileMetadata, error) {
	pbw := &placementBlockWriter{
		client:   c,
		replicas: make(map[datanode.BlockID][]namenode.BlockReplica),
	}

	writer, err := NewWriter(c.blockSize, pbw)
	if err != nil {
		return namenode.FileMetadata{}, err
	}

	result, err := writer.Write(ctx, r)
	if err != nil {
		return namenode.FileMetadata{}, err
	}

	blocks := make([]namenode.BlockMetadata, 0, len(result.Blocks))
	for _, info := range result.Blocks {
		blocks = append(blocks, namenode.BlockMetadata{
			Info:     info,
			Replicas: pbw.replicas[info.ID],
		})
	}

	meta := namenode.FileMetadata{
		Path:   path,
		Size:   result.Size,
		Blocks: blocks,
	}

	return c.metadata.PutFile(ctx, meta)
}

type placementBlockWriter struct {
	client   *DFSClient
	replicas map[datanode.BlockID][]namenode.BlockReplica
}

func (w *placementBlockWriter) PutBlock(ctx context.Context, id datanode.BlockID, r io.Reader) (datanode.BlockInfo, error) {
	data, err := io.ReadAll(r)
	if err != nil {
		return datanode.BlockInfo{}, err
	}

	nodes, err := w.client.metadata.AllocateBlock(ctx, uint64(len(data)), w.client.replicas)
	if err != nil {
		return datanode.BlockInfo{}, err
	}
	if len(nodes) == 0 {
		return datanode.BlockInfo{}, ErrNoAllocatedDataNodes
	}

	replicas := make([]namenode.BlockReplica, 0, len(nodes))

	var info datanode.BlockInfo
	for i, node := range nodes {
		blockClient := w.client.blocks(node.Addr)
		if blockClient == nil {
			return datanode.BlockInfo{}, ErrNilBlockClient
		}

		written, err := blockClient.PutBlock(ctx, id, bytes.NewReader(data))
		if err != nil {
			return datanode.BlockInfo{}, err
		}

		replicas = append(replicas, namenode.BlockReplica{
			NodeID: node.ID,
			Addr:   node.Addr,
		})

		if i == 0 {
			info = written
		}
	}

	w.replicas[id] = replicas

	return info, nil
}
func (c *DFSClient) GetFile(ctx context.Context, path namenode.FilePath, dst io.Writer) (int64, error) {
	meta, err := c.metadata.GetFile(ctx, path)
	if err != nil {
		return 0, err
	}

	blockInfos := make([]datanode.BlockInfo, 0, len(meta.Blocks))
	for _, block := range meta.Blocks {
		blockInfos = append(blockInfos, block.Info)
	}

	reader, err := NewReader(c.blocks(c.defaultBlockAddr))
	if err != nil {
		return 0, err
	}

	return reader.Read(ctx, blockInfos, dst)
}

func (c *DFSClient) StatFile(ctx context.Context, path namenode.FilePath) (namenode.FileMetadata, error) {
	return c.metadata.GetFile(ctx, path)
}

func (c *DFSClient) DeleteFile(ctx context.Context, path namenode.FilePath) error {
	return c.metadata.DeleteFile(ctx, path)
}
