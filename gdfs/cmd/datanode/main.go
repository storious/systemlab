package main

import (
	"context"
	"flag"
	"log"
	"net/http"
	"time"

	"gdfs/internal/cluster"
	"gdfs/internal/datanode"
	"gdfs/internal/namenode"
)

func main() {
	var (
		id           = flag.String("id", "node-1", "datanode id")
		addr         = flag.String("addr", ":9001", "listen address")
		root         = flag.String("root", "data/datanode", "storage root")
		namenodeAddr = flag.String("namenode", "", "namenode address")
		capacity     = flag.Uint64("capacity", 1024*1024*1024, "datanode capacity in bytes")
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

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	hb := cluster.Heartbeat{
		ID:       cluster.DataNodeID(*id),
		Addr:     "http://localhost" + *addr,
		Capacity: *capacity,
		Used:     0,
	}

	startHeartbeat(ctx, *namenodeAddr, hb, 5*time.Second)

	server := datanode.NewHTTPServer(node)

	log.Printf("starting datanode id=%s addr=%s root=%s", *id, *addr, *root)

	if err := http.ListenAndServe(*addr, server); err != nil {
		log.Fatal(err)
	}
}

func startHeartbeat(ctx context.Context, namenodeAddr string, hb cluster.Heartbeat, interval time.Duration) {
	if namenodeAddr == "" {
		return
	}

	client := namenode.NewHTTPClient(namenodeAddr)

	go func() {
		ticker := time.NewTicker(interval)
		defer ticker.Stop()

		send := func() {
			if err := client.Heartbeat(ctx, hb); err != nil {
				log.Printf("heartbeat failed: %v", err)
				return
			}
			log.Printf("heartbeat sent id=%s namenode=%s", hb.ID, namenodeAddr)
		}

		send()

		for {
			select {
			case <-ctx.Done():
				return
			case <-ticker.C:
				send()
			}
		}
	}()
}
