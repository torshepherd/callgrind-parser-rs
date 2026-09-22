# callgrind-parser

> **Experimental — documentation status:** This crate is experimental. Its
> documentation is fully LLM-generated and may contain errors or outdated claims.
> The documentation will receive a review and cleanup pass before the 1.0 release.

A Rust library for reading existing Callgrind-format profiles. It provides an
incremental `Decoder<BufRead>` and the owned `parse_reader` and `parse_profile`
APIs, backed by one grammar and state machine.

The model preserves qualified function identities, per-part event layouts,
self costs, inclusive call-edge costs and jump counts separately. The streaming
decoder does not retain past cost records. Full Callgrind dialect support is
not claimed; this crate does not collect profiles or implement Valgrind.

Part of [callgrind-parser-rs](https://github.com/torshepherd/callgrind-parser-rs).

