# pprof-profile

> **Experimental — documentation status:** This crate is experimental. Its
> documentation is fully LLM-generated and may contain errors or outdated claims.
> The documentation will receive a review and cleanup pass before the 1.0 release.

Shared pprof protobuf bindings and bounded, validated raw/gzip I/O for the
Callgrind conversion tools. The schema and Rust bindings are checked in; an
ordinary Cargo build does not require `protoc` or system zlib.

The library validates profile structure and references, bounds input sizes,
and writes deterministic gzip headers. Conversion policies, including signed
cost handling, belong to the consuming converter. The upstream schema's license
is included in `LICENSE-schema`.

Part of [callgrind-parser-rs](https://github.com/torshepherd/callgrind-parser-rs).

