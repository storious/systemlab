package namenode

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"

	"gdfs/internal/cluster"
)

type HTTPClient struct {
	baseURL string
	client  *http.Client
}

func NewHTTPClient(baseURL string) *HTTPClient {
	return &HTTPClient{
		baseURL: strings.TrimRight(baseURL, "/"),
		client:  http.DefaultClient,
	}
}

func (c *HTTPClient) PutFile(ctx context.Context, meta FileMetadata) (FileMetadata, error) {
	body, err := json.Marshal(meta)
	if err != nil {
		return FileMetadata{}, err
	}

	req, err := http.NewRequestWithContext(
		ctx,
		http.MethodPut,
		c.fileURL(meta.Path),
		bytes.NewReader(body),
	)
	if err != nil {
		return FileMetadata{}, err
	}

	resp, err := c.client.Do(req)
	if err != nil {
		return FileMetadata{}, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusCreated {
		return FileMetadata{}, fmt.Errorf("put file failed: status=%s", resp.Status)
	}

	var out FileMetadata
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return FileMetadata{}, err
	}

	return out, nil
}

func (c *HTTPClient) GetFile(ctx context.Context, path FilePath) (FileMetadata, error) {
	req, err := http.NewRequestWithContext(
		ctx,
		http.MethodGet,
		c.fileURL(path),
		nil,
	)
	if err != nil {
		return FileMetadata{}, err
	}

	resp, err := c.client.Do(req)
	if err != nil {
		return FileMetadata{}, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return FileMetadata{}, fmt.Errorf("get file failed: status=%s", resp.Status)
	}

	var meta FileMetadata
	if err := json.NewDecoder(resp.Body).Decode(&meta); err != nil {
		return FileMetadata{}, err
	}

	return meta, nil
}

func (c *HTTPClient) DeleteFile(ctx context.Context, path FilePath) error {
	req, err := http.NewRequestWithContext(
		ctx,
		http.MethodDelete,
		c.fileURL(path),
		nil,
	)
	if err != nil {
		return err
	}

	resp, err := c.client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNoContent {
		return fmt.Errorf("delete file failed: status=%s", resp.Status)
	}

	return nil
}

func (c *HTTPClient) fileURL(path FilePath) string {
	clean := strings.TrimPrefix(string(path), "/")
	return c.baseURL + "/files/" + clean
}

func (c *HTTPClient) AllocateBlock(ctx context.Context, blockSize uint64, replicas int) ([]cluster.DataNodeInfo, error) {
	body, err := json.Marshal(AllocateBlockRequest{
		BlockSize: blockSize,
		Replicas:  replicas,
	})
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequestWithContext(
		ctx,
		http.MethodPost,
		c.baseURL+"/blocks/allocate",
		bytes.NewReader(body),
	)
	if err != nil {
		return nil, err
	}

	resp, err := c.client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("allocate block failed: status=%s", resp.Status)
	}

	var out AllocateBlockResponse
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return nil, err
	}

	return out.DataNodes, nil
}
