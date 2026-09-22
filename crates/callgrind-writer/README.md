# callgrind-writer

> **Experimental — documentation status:** This crate is experimental. Its
> documentation is fully LLM-generated and may contain errors or outdated claims.
> The documentation will receive a review and cleanup pass before the 1.0 release.

A small streaming Callgrind writer used by `pprof2callgrind`.

It writes one part with instruction and line positions, explicit function
identities, exclusive costs, call edges and checked self totals. It does not
support derived events, jumps or relative instruction positions. Call-edge
costs remain separate from self totals.

Part of [callgrind-parser-rs](https://github.com/torshepherd/callgrind-parser-rs).

