package main

import (
	"flag"
	"log"
	"net/http"

	"gdfs/internal/datanode"
)

func main() {
	var (
		id   = flag.String("id", "node-1", "datanode id")
		addr = flag.String("addr", ":9001", "listen address")
		root = flag.String("root", "data/datanode", "storage root")
	)
	flag.Parse()

	store := datanode.NewLocalBlockStore(*root)

	node, err := datanode.NewDataNode(
		datanode.NodeID(*id),
		*addr,
		store,
	)
	if err != nil {
		log.Fatal(err)
	}

	server := datanode.NewHTTPServer(node)

	log.Printf("starting datanode id=%s addr=%s root=%s", *id, *addr, *root)

	if err := http.ListenAndServe(*addr, server); err != nil {
		log.Fatal(err)
	}
}
