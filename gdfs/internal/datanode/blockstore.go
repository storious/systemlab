package datanode

import (
	"errors"
	"io"
	"os"
	"path/filepath"
)

type BlockStore interface {
	WriteBlock(blockID string, r io.Reader) (int64, error)
	ReadBlock(blockID string) (int64, io.ReadCloser, error)
	HasBlock(blockID string) bool
	DeleteBlock(blockID string) error
}

type LocalBlockStore struct {
	root string
}

func NewLocalBlockStore(root string) *LocalBlockStore {
	return &LocalBlockStore{root: root}
}

func (s *LocalBlockStore) WriteBlock(blockID string, r io.Reader) (int64, error) {
	path := s.blockPath(blockID)

	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return 0, err
	}

	f, err := os.Create(path)
	if err != nil {
		return 0, err
	}
	defer f.Close()

	return io.Copy(f, r)
}

func (s *LocalBlockStore) ReadBlock(blockID string) (int64, io.ReadCloser, error) {
	path := s.blockPath(blockID)

	f, err := os.Open(path)
	if err != nil {
		return 0, nil, err
	}

	info, err := f.Stat()
	if err != nil {
		f.Close()
		return 0, nil, err
	}

	return info.Size(), f, nil
}

func (s *LocalBlockStore) HasBlock(blockID string) bool {
	_, err := os.Stat(s.blockPath(blockID))
	return !errors.Is(err, os.ErrNotExist)
}

func (s *LocalBlockStore) DeleteBlock(blockID string) error {
	return os.Remove(s.blockPath(blockID))
}

func (s *LocalBlockStore) blockPath(blockID string) string {
	return filepath.Join(s.root, "blocks", blockID)
}
