package datanode

import (
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/stretchr/testify/require"
)

func newTestHTTPServer(t *testing.T) *httptest.Server {
	t.Helper()

	store := NewLocalBlockStore(t.TempDir())

	node, err := NewDataNode("node-1", "127.0.0.1:0", store)
	require.NoError(t, err)

	return httptest.NewServer(NewHTTPServer(node))
}

func TestHTTPServerPutGetBlock(t *testing.T) {
	server := newTestHTTPServer(t)
	defer server.Close()

	const blockID = "block-001"
	const content = "hello datanode http"

	req, err := http.NewRequest(
		http.MethodPut,
		server.URL+"/blocks/"+blockID,
		strings.NewReader(content),
	)
	require.NoError(t, err)

	resp, err := http.DefaultClient.Do(req)
	require.NoError(t, err)
	defer resp.Body.Close()

	require.Equal(t, http.StatusCreated, resp.StatusCode)

	getResp, err := http.Get(server.URL + "/blocks/" + blockID)
	require.NoError(t, err)
	defer getResp.Body.Close()

	require.Equal(t, http.StatusOK, getResp.StatusCode)

	body, err := io.ReadAll(getResp.Body)
	require.NoError(t, err)
	require.Equal(t, content, string(body))
}

func TestHTTPServerHeadBlock(t *testing.T) {
	server := newTestHTTPServer(t)
	defer server.Close()

	const blockID = "block-001"
	const content = "hello"

	putReq, err := http.NewRequest(
		http.MethodPut,
		server.URL+"/blocks/"+blockID,
		strings.NewReader(content),
	)
	require.NoError(t, err)

	putResp, err := http.DefaultClient.Do(putReq)
	require.NoError(t, err)
	defer putResp.Body.Close()

	require.Equal(t, http.StatusCreated, putResp.StatusCode)

	headReq, err := http.NewRequest(http.MethodHead, server.URL+"/blocks/"+blockID, nil)
	require.NoError(t, err)

	headResp, err := http.DefaultClient.Do(headReq)
	require.NoError(t, err)
	defer headResp.Body.Close()

	require.Equal(t, http.StatusOK, headResp.StatusCode)
	require.Equal(t, "5", headResp.Header.Get("X-Block-Size"))
	require.NotEmpty(t, headResp.Header.Get("X-Block-Checksum"))
}

func TestHTTPServerDeleteBlock(t *testing.T) {
	server := newTestHTTPServer(t)
	defer server.Close()

	const blockID = "block-001"

	putReq, err := http.NewRequest(
		http.MethodPut,
		server.URL+"/blocks/"+blockID,
		strings.NewReader("hello"),
	)
	require.NoError(t, err)

	putResp, err := http.DefaultClient.Do(putReq)
	require.NoError(t, err)
	defer putResp.Body.Close()

	require.Equal(t, http.StatusCreated, putResp.StatusCode)

	deleteReq, err := http.NewRequest(http.MethodDelete, server.URL+"/blocks/"+blockID, nil)
	require.NoError(t, err)

	deleteResp, err := http.DefaultClient.Do(deleteReq)
	require.NoError(t, err)
	defer deleteResp.Body.Close()

	require.Equal(t, http.StatusNoContent, deleteResp.StatusCode)

	getResp, err := http.Get(server.URL + "/blocks/" + blockID)
	require.NoError(t, err)
	defer getResp.Body.Close()

	require.Equal(t, http.StatusNotFound, getResp.StatusCode)
}

func TestHTTPServerMissingBlock(t *testing.T) {
	server := newTestHTTPServer(t)
	defer server.Close()

	resp, err := http.Get(server.URL + "/blocks/missing-block")
	require.NoError(t, err)
	defer resp.Body.Close()

	require.Equal(t, http.StatusNotFound, resp.StatusCode)
}

func TestHTTPServerMethodNotAllowed(t *testing.T) {
	server := newTestHTTPServer(t)
	defer server.Close()

	req, err := http.NewRequest(http.MethodPost, server.URL+"/blocks/block-001", nil)
	require.NoError(t, err)

	resp, err := http.DefaultClient.Do(req)
	require.NoError(t, err)
	defer resp.Body.Close()

	require.Equal(t, http.StatusMethodNotAllowed, resp.StatusCode)
}

func TestHTTPServerMissingBlockID(t *testing.T) {
	server := newTestHTTPServer(t)
	defer server.Close()

	resp, err := http.Get(server.URL + "/blocks/")
	require.NoError(t, err)
	defer resp.Body.Close()

	require.Equal(t, http.StatusBadRequest, resp.StatusCode)
}
