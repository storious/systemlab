package main

import (
	"flag"
	"log"
	"net/http"

	"gdfs/internal/namenode"
)

func main() {
	addr := flag.String("addr", ":9000", "listen address")
	flag.Parse()

	node, err := namenode.NewNameNode(namenode.NewMetadataStore())
	if err != nil {
		log.Fatal(err)
	}

	server := namenode.NewHTTPServer(node)

	log.Printf("starting namenode addr=%s", *addr)

	if err := http.ListenAndServe(*addr, server); err != nil {
		log.Fatal(err)
	}
}
