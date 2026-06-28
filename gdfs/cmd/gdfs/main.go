package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"os"

	"gdfs/internal/client"
	"gdfs/internal/datanode"
	"gdfs/internal/namenode"
)

func main() {
	var (
		namenodeAddr = flag.String("namenode", "http://localhost:9000", "namenode address")
		datanodeAddr = flag.String("datanode", "http://localhost:9001", "datanode address")
		blockSize    = flag.Int64("block-size", 64*1024*1024, "block size in bytes")
	)
	flag.Parse()

	if flag.NArg() < 1 {
		usage()
		os.Exit(1)
	}

	blockClient := datanode.NewHTTPClient(*datanodeAddr)
	metaClient := namenode.NewHTTPClient(*namenodeAddr)

	fs, err := client.NewDFSClient(*blockSize, blockClient, metaClient)
	if err != nil {
		log.Fatal(err)
	}

	ctx := context.Background()

	switch flag.Arg(0) {
	case "put":
		if flag.NArg() != 3 {
			log.Fatal("usage: gdfs put <local-path> <gdfs-path>")
		}
		put(ctx, fs, flag.Arg(1), flag.Arg(2))

	case "get":
		if flag.NArg() != 3 {
			log.Fatal("usage: gdfs get <gdfs-path> <local-path>")
		}
		get(ctx, fs, flag.Arg(1), flag.Arg(2))

	case "stat":
		if flag.NArg() != 2 {
			log.Fatal("usage: gdfs stat <gdfs-path>")
		}
		stat(ctx, fs, flag.Arg(1))

	case "delete":
		if flag.NArg() != 2 {
			log.Fatal("usage: gdfs delete <gdfs-path>")
		}
		del(ctx, fs, flag.Arg(1))

	default:
		usage()
		os.Exit(1)
	}
}

func put(ctx context.Context, fs *client.DFSClient, localPath, remotePath string) {
	f, err := os.Open(localPath)
	if err != nil {
		log.Fatal(err)
	}
	defer f.Close()

	meta, err := fs.PutFile(ctx, namenode.FilePath(remotePath), f)
	if err != nil {
		log.Fatal(err)
	}

	fmt.Printf("stored %s size=%d blocks=%d\n", meta.Path, meta.Size, len(meta.Blocks))
}

func get(ctx context.Context, fs *client.DFSClient, remotePath, localPath string) {
	f, err := os.Create(localPath)
	if err != nil {
		log.Fatal(err)
	}
	defer f.Close()

	n, err := fs.GetFile(ctx, namenode.FilePath(remotePath), f)
	if err != nil {
		log.Fatal(err)
	}

	fmt.Printf("retrieved %s size=%d\n", remotePath, n)
}

func stat(ctx context.Context, fs *client.DFSClient, remotePath string) {
	meta, err := fs.StatFile(ctx, namenode.FilePath(remotePath))
	if err != nil {
		log.Fatal(err)
	}

	fmt.Printf("path: %s\n", meta.Path)
	fmt.Printf("size: %d\n", meta.Size)
	fmt.Printf("blocks: %d\n", len(meta.Blocks))

	for i, block := range meta.Blocks {
		fmt.Printf("  [%d] id=%s size=%d checksum=%s\n", i, block.ID, block.Size, block.Checksum)
	}
}

func del(ctx context.Context, fs *client.DFSClient, remotePath string) {
	if err := fs.DeleteFile(ctx, namenode.FilePath(remotePath)); err != nil {
		log.Fatal(err)
	}

	fmt.Printf("deleted %s\n", remotePath)
}

func usage() {
	fmt.Println(`usage:
  gdfs [flags] put <local-path> <gdfs-path>
  gdfs [flags] get <gdfs-path> <local-path>
  gdfs [flags] stat <gdfs-path>
  gdfs [flags] delete <gdfs-path>

flags:
  -namenode <url>   namenode address, default http://localhost:9000
  -datanode <url>   datanode address, default http://localhost:9001
  -block-size <n>   block size in bytes`)
}
