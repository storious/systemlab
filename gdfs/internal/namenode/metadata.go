package namenode

import (
	"errors"
	"sync"

	"gdfs/internal/cluster"
	"gdfs/internal/datanode"
)

type FilePath string

type BlockReplica struct {
	NodeID cluster.DataNodeID `json:"node_id"`
	Addr   string             `json:"addr"`
}

type BlockMetadata struct {
	Info     datanode.BlockInfo `json:"info"`
	Replicas []BlockReplica     `json:"replicas"`
}

type FileMetadata struct {
	Path   FilePath        `json:"path"`
	Size   int64           `json:"size"`
	Blocks []BlockMetadata `json:"blocks"`
}

type MetadataStore struct {
	mu    sync.RWMutex
	files map[FilePath]FileMetadata
}

func NewMetadataStore() *MetadataStore {
	return &MetadataStore{
		files: make(map[FilePath]FileMetadata),
	}
}

func (s *MetadataStore) PutFile(meta FileMetadata) error {
	if meta.Path == "" {
		return errors.New("empty file path")
	}

	s.mu.Lock()
	defer s.mu.Unlock()

	s.files[meta.Path] = meta
	return nil
}

func (s *MetadataStore) GetFile(path FilePath) (FileMetadata, error) {
	if path == "" {
		return FileMetadata{}, errors.New("empty file path")
	}

	s.mu.RLock()
	defer s.mu.RUnlock()

	meta, ok := s.files[path]
	if !ok {
		return FileMetadata{}, errors.New("file not found")
	}

	return meta, nil
}

func (s *MetadataStore) DeleteFile(path FilePath) error {
	if path == "" {
		return errors.New("empty file path")
	}

	s.mu.Lock()
	defer s.mu.Unlock()

	delete(s.files, path)
	return nil
}

func (s *MetadataStore) Exists(path FilePath) bool {
	s.mu.RLock()
	defer s.mu.RUnlock()

	_, ok := s.files[path]
	return ok
}
