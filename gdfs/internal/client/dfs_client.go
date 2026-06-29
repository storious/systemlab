package client

import (
	"context"
	"io"

	"gdfs/internal/cluster"
	"gdfs/internal/namenode"
)

type MetadataClient interface {
	AllocateBlock(ctx context.Context, blockSize uint64, replicas int) ([]cluster.DataNodeInfo, error)

	PutFile(ctx context.Context, meta namenode.FileMetadata) (namenode.FileMetadata, error)
	GetFile(ctx context.Context, path namenode.FilePath) (namenode.FileMetadata, error)
	DeleteFile(ctx context.Context, path namenode.FilePath) error
}

type DFSClient struct {
	writer   *Writer
	reader   *Reader
	metadata MetadataClient
}

type BlockClient interface {
	BlockWriter
	BlockReader
}

func NewDFSClient(blockSize int64, blocks BlockClient, metadata MetadataClient) (*DFSClient, error) {
	if blocks == nil {
		return nil, ErrNilBlockClient
	}
	if metadata == nil {
		return nil, ErrNilMetadataClient
	}

	writer, err := NewWriter(blockSize, blocks)
	if err != nil {
		return nil, err
	}

	reader, err := NewReader(blocks)
	if err != nil {
		return nil, err
	}

	return &DFSClient{
		writer:   writer,
		reader:   reader,
		metadata: metadata,
	}, nil
}

func (c *DFSClient) PutFile(ctx context.Context, path namenode.FilePath, r io.Reader) (namenode.FileMetadata, error) {
	result, err := c.writer.Write(ctx, r)
	if err != nil {
		return namenode.FileMetadata{}, err
	}

	meta := namenode.FileMetadata{
		Path:   path,
		Size:   result.Size,
		Blocks: result.Blocks,
	}

	return c.metadata.PutFile(ctx, meta)
}

func (c *DFSClient) GetFile(ctx context.Context, path namenode.FilePath, dst io.Writer) (int64, error) {
	meta, err := c.metadata.GetFile(ctx, path)
	if err != nil {
		return 0, err
	}

	return c.reader.Read(ctx, meta.Blocks, dst)
}

func (c *DFSClient) StatFile(ctx context.Context, path namenode.FilePath) (namenode.FileMetadata, error) {
	return c.metadata.GetFile(ctx, path)
}

func (c *DFSClient) DeleteFile(ctx context.Context, path namenode.FilePath) error {
	return c.metadata.DeleteFile(ctx, path)
}
