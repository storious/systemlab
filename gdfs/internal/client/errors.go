package client

import "errors"

var (
	ErrNilBlockReader        = errors.New("nil block reader")
	ErrNilBlockWriter        = errors.New("nil block writer")
	ErrNilBlockClient        = errors.New("nil block client")
	ErrNilMetadataClient     = errors.New("nil metadata client")
	ErrInvalidBlockSize      = errors.New("invalid block size")
	ErrInvalidReplicaCount   = errors.New("invalid replica count")
	ErrNilBlockClientFactory = errors.New("nil block client factory")
	ErrNoAllocatedDataNodes  = errors.New("no allocated datanodes")
)
