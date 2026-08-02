# Third-party notices

This repository is a native Rust source port of `sergi/go-diff`, pinned to tag
`v1.4.0`, commit `57c41f4cb9849a2e83cdbd7644b31e6d7a7e2586`.

The Go implementation is Copyright (c) 2012–2016 The go-diff Authors and is
distributed under the MIT License. Its exact `LICENSE`, `AUTHORS`, and
`CONTRIBUTORS` files are preserved at the repository root and inside
`tests/original/go-diff/`.

The underlying Diff, Match and Patch algorithm was written by Neil Fraser,
Copyright (c) 2006 Google Inc., and is distributed under the Apache License
2.0. The license text is preserved as `APACHE-LICENSE-2.0`.

No source-language runtime, FFI bridge, proxy, or pre-existing Rust port is
used by the Rust implementation. Go is used only to execute the pinned
reference implementation in validation, differential testing, and benchmark
tooling.
