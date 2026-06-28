package datanode

import (
	"context"
	"encoding/json"
	"net/http"
	"strconv"
	"strings"
)

type HTTPServer struct {
	node *DataNode
	mux  *http.ServeMux
}

func NewHTTPServer(node *DataNode) *HTTPServer {
	s := &HTTPServer{
		node: node,
		mux:  http.NewServeMux(),
	}
	s.routes()
	return s
}

func (s *HTTPServer) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	s.mux.ServeHTTP(w, r)
}

func (s *HTTPServer) routes() {
	s.mux.HandleFunc("/blocks/", s.handleBlock)
}

func (s *HTTPServer) handleBlock(w http.ResponseWriter, r *http.Request) {
	id := BlockID(strings.TrimPrefix(r.URL.Path, "/blocks/"))
	if id == "" {
		http.Error(w, "missing block id", http.StatusBadRequest)
		return
	}

	switch r.Method {
	case http.MethodPut:
		s.handlePutBlock(w, r, id)
	case http.MethodGet:
		s.handleGetBlock(w, r, id)
	case http.MethodHead:
		s.handleHeadBlock(w, r, id)
	case http.MethodDelete:
		s.handleDeleteBlock(w, r, id)
	default:
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
	}
}

func (s *HTTPServer) handlePutBlock(w http.ResponseWriter, r *http.Request, id BlockID) {
	info, err := s.node.PutBlock(r.Context(), &Block{
		ID:   id,
		Data: r.Body,
	})
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	writeJSON(w, http.StatusCreated, info)
}

func (s *HTTPServer) handleGetBlock(w http.ResponseWriter, r *http.Request, id BlockID) {
	block, err := s.node.GetBlock(r.Context(), id)
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}

	if rc, ok := block.Data.(interface{ Close() error }); ok {
		defer rc.Close()
	}

	w.WriteHeader(http.StatusOK)
	_ = http.NewResponseController(w).Flush()
	_, _ = copyWithContext(r.Context(), w, block.Data)
}

func (s *HTTPServer) handleHeadBlock(w http.ResponseWriter, _ *http.Request, id BlockID) {
	info, err := s.node.StatBlock(context.Background(), id)
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}

	w.Header().Set("X-Block-Checksum", info.Checksum)
	w.Header().Set("X-Block-Size", strconv.FormatInt(info.Size, 10))
	w.WriteHeader(http.StatusOK)
}

func (s *HTTPServer) handleDeleteBlock(w http.ResponseWriter, r *http.Request, id BlockID) {
	if err := s.node.DeleteBlock(r.Context(), id); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}
