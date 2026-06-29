## GDFS

- Cluster Management Lives in NameNode (v0.2)

- MetaData Model
    - FileMetadata owns ordered block metadata.
    - BlockMetadata owns block identity, size, checksum, and replica locations.
    - DFSClient must use metadata-driven reads instead of default DataNode reads.
    -Placement produces replica candidates; committed metadata records actual replicas.
