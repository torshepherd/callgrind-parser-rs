# Shared pprof wire schema

`profile.proto` is the unmodified Apache-2.0 schema from google/pprof commit
`6331bc6350fe55a6fec2957299e0581dd7510e36`; its license is `LICENSE-schema`.
Source: https://github.com/google/pprof/blob/6331bc6350fe55a6fec2957299e0581dd7510e36/proto/profile.proto

`src/proto.rs` is a hand-maintained Prost transcription of every message and
field, including columns and doc_url. Ordinary builds need neither protoc nor
Go nor system zlib. Update schema and bindings together. Unit tests exercise
all fields; the separate pprof CI gate cross-reads real upstream-generated
profiles and Rust-reencoded gzip with the pinned upstream Go reader.

The reader accepts raw protobuf or exactly one gzip member, with a configurable
limit on both compressed and decoded bytes (default 256 MiB). This is not a
total process-memory guarantee: protobuf objects and conversion indexes need
additional memory. Unknown protobuf fields are accepted but not retained.
IDs, references, string indexes and sample-vector widths are checked. Signed
values remain valid here; each converter chooses its own representability
policy. Output gzip has deterministic metadata and propagates finalization
errors. No legacy text profiles, symbolization or binary lookup is performed.
