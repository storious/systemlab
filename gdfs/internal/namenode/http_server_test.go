package namenode

import (
	"bytes"
	"context"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"testing"

	"gdfs/internal/cluster"
	"gdfs/internal/datanode"

	"github.com/stretchr/testify/require"
)

func newTestHTTPServer(t *testing.T) *httptest.Server {
	t.Helper()

	node, err := NewNameNode(NewMetadataStore())
	require.NoError(t, err)

	return httptest.NewServer(NewHTTPServer(node))
}

func TestHTTPServerPutGetFile(t *testing.T) {
	server := newTestHTTPServer(t)
	defer server.Close()

	meta := FileMetadata{
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

	body, err := json.Marshal(meta)
	require.NoError(t, err)

	putResp, err := http.DefaultClient.Do(mustRequest(
		t,
		http.MethodPut,
		server.URL+"/files/docs/hello.txt",
		bytes.NewReader(body),
	))
	require.NoError(t, err)
	defer putResp.Body.Close()

	require.Equal(t, http.StatusCreated, putResp.StatusCode)

	getResp, err := http.Get(server.URL + "/files/docs/hello.txt")
	require.NoError(t, err)
	defer getResp.Body.Close()

	require.Equal(t, http.StatusOK, getResp.StatusCode)

	var got FileMetadata
	err = json.NewDecoder(getResp.Body).Decode(&got)
	require.NoError(t, err)

	require.Equal(t, FilePath("/docs/hello.txt"), got.Path)
	require.Equal(t, meta.Size, got.Size)
	require.Equal(t, meta.Blocks, got.Blocks)
}

func TestHTTPServerDeleteFile(t *testing.T) {
	server := newTestHTTPServer(t)
	defer server.Close()

	body, err := json.Marshal(FileMetadata{
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

	putResp, err := http.DefaultClient.Do(mustRequest(
		t,
		http.MethodPut,
		server.URL+"/files/docs/hello.txt",
		bytes.NewReader(body),
	))
	require.NoError(t, err)
	defer putResp.Body.Close()

	require.Equal(t, http.StatusCreated, putResp.StatusCode)

	deleteResp, err := http.DefaultClient.Do(mustRequest(
		t,
		http.MethodDelete,
		server.URL+"/files/docs/hello.txt",
		nil,
	))
	require.NoError(t, err)
	defer deleteResp.Body.Close()

	require.Equal(t, http.StatusNoContent, deleteResp.StatusCode)

	getResp, err := http.Get(server.URL + "/files/docs/hello.txt")
	require.NoError(t, err)
	defer getResp.Body.Close()

	require.Equal(t, http.StatusNotFound, getResp.StatusCode)
}

func TestHTTPServerMissingFilePath(t *testing.T) {
	server := newTestHTTPServer(t)
	defer server.Close()

	resp, err := http.Get(server.URL + "/files/")
	require.NoError(t, err)
	defer resp.Body.Close()

	require.Equal(t, http.StatusBadRequest, resp.StatusCode)
}

func TestHTTPServerMethodNotAllowed(t *testing.T) {
	server := newTestHTTPServer(t)
	defer server.Close()

	resp, err := http.DefaultClient.Do(mustRequest(
		t,
		http.MethodPost,
		server.URL+"/files/docs/hello.txt",
		nil,
	))
	require.NoError(t, err)
	defer resp.Body.Close()

	require.Equal(t, http.StatusMethodNotAllowed, resp.StatusCode)
}

func mustRequest(t *testing.T, method, url string, body io.Reader) *http.Request {
	t.Helper()

	req, err := http.NewRequest(method, url, body)
	require.NoError(t, err)
	return req
}

func TestHTTPServerAllocateBlock(t *testing.T) {
	node, err := NewNameNode(NewMetadataStore())
	require.NoError(t, err)

	ctx := context.Background()

	require.NoError(t, node.RegisterDataNode(ctx, cluster.DataNodeInfo{
		ID:       "node-1",
		Addr:     "http://localhost:9001",
		Capacity: 1000,
		Used:     900,
	}))

	require.NoError(t, node.RegisterDataNode(ctx, cluster.DataNodeInfo{
		ID:       "node-2",
		Addr:     "http://localhost:9002",
		Capacity: 1000,
		Used:     100,
	}))

	server := httptest.NewServer(NewHTTPServer(node))
	defer server.Close()

	body, err := json.Marshal(AllocateBlockRequest{
		BlockSize: 100,
		Replicas:  1,
	})
	require.NoError(t, err)

	resp, err := http.DefaultClient.Do(mustRequest(
		t,
		http.MethodPost,
		server.URL+"/blocks/allocate",
		bytes.NewReader(body),
	))
	require.NoError(t, err)
	defer resp.Body.Close()

	require.Equal(t, http.StatusOK, resp.StatusCode)

	var out AllocateBlockResponse
	err = json.NewDecoder(resp.Body).Decode(&out)
	require.NoError(t, err)

	require.Len(t, out.DataNodes, 1)
	require.Equal(t, cluster.DataNodeID("node-2"), out.DataNodes[0].ID)
}
