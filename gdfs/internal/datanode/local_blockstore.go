package datanode

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
)

type LocalBlockStore struct {
	root string
}

func NewLocalBlockStore(root string) *LocalBlockStore {
	return &LocalBlockStore{root: root}
}

func (s *LocalBlockStore) Put(ctx context.Context, block *Block) (BlockInfo, error) {
	if block == nil {
		return BlockInfo{}, errors.New("nil block")
	}
	if block.ID == "" {
		return BlockInfo{}, errors.New("empty block id")
	}
	if block.Data == nil {
		return BlockInfo{}, errors.New("nil block data")
	}

	path := s.blockPath(block.ID)
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return BlockInfo{}, err
	}

	tmpPath := path + ".tmp"
	_ = os.Remove(tmpPath)

	f, err := os.Create(tmpPath)
	if err != nil {
		return BlockInfo{}, err
	}

	committed := false
	defer func() {
		_ = f.Close()
		if !committed {
			_ = os.Remove(tmpPath)
		}
	}()

	hasher := sha256.New()
	w := io.MultiWriter(f, hasher)

	n, err := copyWithContext(ctx, w, block.Data)
	if err != nil {
		return BlockInfo{}, err
	}

	if err := f.Close(); err != nil {
		return BlockInfo{}, err
	}

	if err := os.Rename(tmpPath, path); err != nil {
		return BlockInfo{}, err
	}

	committed = true

	return BlockInfo{
		ID:       block.ID,
		Size:     n,
		Checksum: hex.EncodeToString(hasher.Sum(nil)),
	}, nil
}

func (s *LocalBlockStore) Get(ctx context.Context, id BlockID) (*Block, error) {
	if id == "" {
		return nil, errors.New("empty block id")
	}

	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	default:
	}

	f, err := os.Open(s.blockPath(id))
	if err != nil {
		return nil, err
	}

	return &Block{
		ID:   id,
		Data: f,
	}, nil
}

func (s *LocalBlockStore) Delete(ctx context.Context, id BlockID) error {
	if id == "" {
		return errors.New("empty block id")
	}

	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
	}

	err := os.Remove(s.blockPath(id))
	if errors.Is(err, os.ErrNotExist) {
		return nil
	}
	return err
}

func (s *LocalBlockStore) Exists(ctx context.Context, id BlockID) bool {
	if id == "" {
		return false
	}

	select {
	case <-ctx.Done():
		return false
	default:
	}

	_, err := os.Stat(s.blockPath(id))
	return err == nil
}

func (s *LocalBlockStore) Stat(ctx context.Context, id BlockID) (BlockInfo, error) {
	if id == "" {
		return BlockInfo{}, errors.New("empty block id")
	}

	select {
	case <-ctx.Done():
		return BlockInfo{}, ctx.Err()
	default:
	}

	path := s.blockPath(id)

	f, err := os.Open(path)
	if err != nil {
		return BlockInfo{}, err
	}
	defer f.Close()

	hasher := sha256.New()
	n, err := io.Copy(hasher, f)
	if err != nil {
		return BlockInfo{}, err
	}

	return BlockInfo{
		ID:       id,
		Size:     n,
		Checksum: hex.EncodeToString(hasher.Sum(nil)),
	}, nil
}

func (s *LocalBlockStore) blockPath(id BlockID) string {
	raw := string(id)

	if len(raw) < 4 {
		return filepath.Join(s.root, "blocks", raw)
	}

	return filepath.Join(
		s.root,
		"blocks",
		raw[:2],
		raw[2:4],
		raw,
	)
}

func copyWithContext(ctx context.Context, dst io.Writer, src io.Reader) (int64, error) {
	buf := make([]byte, 32*1024)
	var written int64

	for {
		select {
		case <-ctx.Done():
			return written, ctx.Err()
		default:
		}

		nr, er := src.Read(buf)
		if nr > 0 {
			nw, ew := dst.Write(buf[:nr])
			if nw < 0 || nr < nw {
				return written, fmt.Errorf("invalid write count")
			}
			written += int64(nw)
			if ew != nil {
				return written, ew
			}
			if nr != nw {
				return written, io.ErrShortWrite
			}
		}

		if er != nil {
			if errors.Is(er, io.EOF) {
				break
			}
			return written, er
		}
	}

	return written, nil
}
