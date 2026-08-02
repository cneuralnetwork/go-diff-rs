# Architectural decisions

Source baseline: `sergi/go-diff` v1.4.0 at
`57c41f4cb9849a2e83cdbd7644b31e6d7a7e2586`.

## 1. Native implementation; Go is validation-only

The shipped crate and CLI never load, link, spawn, or proxy the Go runtime. Go
appears only in `tests/original`, `fuzz/go-oracle`, `bench/go-driver`, and their
evidence scripts. This keeps the target artifact a real port while retaining a
strong executable reference during development.

## 2. Freeze one source commit, not a moving branch

All decisions target v1.4.0's exact commit. The untouched archive has both a
test/fixture manifest and a whole-tree manifest. This prevents an upstream
change, regenerated fixture, or quiet test edit from changing the goalposts.

## 3. Represent Go strings with a byte-preserving `Text` type

Go strings permit invalid UTF-8, whereas Rust `String` does not. Converting every
API to `String` would lose source behavior. `Text(Vec<u8>)` preserves bytes,
accepts common Rust string/byte inputs, offers checked `as_str`, and uses lossy
conversion only when the caller explicitly requests display convenience.

## 4. Decode invalid UTF-8 exactly like Go

The diff algorithm operates on code points, so the port includes a small native
decoder matching Go's rule that each invalid input byte becomes U+FFFD. It does
not call a Go helper or rely on Rust's chunk-oriented lossy conversion. The
invalid-byte source tests and shared comparison corpus verify this distinction.

## 5. Use `u32` for Go rune sequences

Rust `char` rejects surrogate values, but a Go `[]rune` can contain any `int32`
value and converts invalid values to U+FFFD when stringified. Public rune APIs
therefore take `&[u32]`; encoding performs Go-compatible validation/replacement.
This is slightly less ergonomic than `char` but preserves the source domain.

## 6. Keep `Operation` open instead of using a closed enum

Go's `type Operation int8` permits values beyond delete/equal/insert, and its
generated String method formats unknown values numerically. A transparent
newtype retains that behavior while constants provide normal typed use. A Rust
enum would make unknown source values unrepresentable.

## 7. Return owned cleanup results

Go slices allow algorithms to splice shared backing storage. Rust cleanup
methods consume and return `Vec<Diff>`, making ownership and mutation explicit
without interior mutability. Test outputs and comparison responses remain
identical; callers gain normal Rust aliasing guarantees. `DiffHalfMatch` uses
`Option<[Text; 5]>` to retain Go's meaningful nil-versus-five-parts result.

## 8. Split variadic `PatchMake` into typed entry points

Go dispatches `...interface{}` at runtime across four accepted call shapes and
silently returns empty output for unsupported shapes. Rust has no safe idiomatic
equivalent to unchecked variadic `any`. The port exposes `patch_make`,
`patch_make_from_diffs`, `patch_make_from_text_and_diffs`, and
`patch_make_deprecated`, preserving all valid functionality with compile-time
types. Every upstream overload test is retained.

## 9. Preserve patch diff privacy but add Rust accessors

Upstream `Patch.diffs` is package-private while coordinate fields are public.
Rust mirrors that boundary and adds borrowed/mutable accessors because external
crate users cannot share Go's package scope. The accessors do not alter patch
serialization or equality.

## 10. Model bisect deadlines explicitly

Go uses the zero value of `time.Time` for infinity. Rust exposes
`Deadline::Infinite` and `Deadline::At(Instant)`, preventing wall-clock/time-zone
ambiguity and making the sentinel explicit. `diff_main` still derives deadlines
from the public timeout configuration exactly as upstream does. The public
timeout is `Duration`; Go's negative durations are not representable, but the
source treats every non-positive value identically, so `Duration::ZERO`
preserves their sole observable algorithmic behavior (no deadline).

## 11. Preserve each API's original index unit

go-diff mixes rune indices in core diff routines with byte offsets in matching,
patch coordinates, and some overlap operations. Normalizing everything to one
unit would be cleaner but behaviorally incompatible. The Rust signatures use
`usize`/`isize` according to each source method, and multibyte tests plus the
index conversion exhaustive test lock the units down.

## 12. Port line identifiers, including the surrogate gap

Line-mode maps unique lines into rune values and deliberately skips U+D800–DFFF.
The native index codec retains that mapping and its maximum-value panic. Using a
plain `char` counter would fail on large line tables and miss the upstream edge
tests.

## 13. Implement Go-compatible URL/query encoding locally

Patch and delta formats depend on Go `url.QueryEscape`, selected encodeURI
unescapes, and exact malformed-escape errors. A generic Rust URL crate would add
a runtime dependency and subtly different escaping. The small internal codec is
covered by original tables and exact comparison responses.

## 14. Zero dependency and zero unsafe policy

The algorithm needs only standard collections, time, formatting, and byte
operations. Keeping `[dependencies]` empty makes the build auditable and avoids
dependency drift. `#![forbid(unsafe_code)]` makes the zero-unsafe bonus a
compiler-enforced invariant, not just a current grep result.

## 15. Preserve the discovered upstream panic in the compatibility profile

Invalid UTF-8 can expand from one byte to the three-byte U+FFFD sequence;
upstream `PatchMake` then slices the original byte string using expanded diff
lengths and panics. Fixing only Rust would create a divergence. The port retains
the panic, adds an explicit regression, isolates it from the continuous survivor
corpus, and publishes a minimized upstream report in `BUG_REPORT.md`.

## 16. Translate tests while retaining the originals untouched

Go test files cannot directly link a Rust crate without a forbidden wrapper.
The original archive is therefore byte-identical and independently runnable,
while all 42 test functions are mechanically translated into native Rust. Every
adaptation is listed in `TEST_ADAPTATIONS.md`, and a script verifies name mapping
and pass rate per file.

## 17. Ship a file-oriented CLI as the runnable artifact

The source package is library-only, but the submission requires a runnable
artifact and shared output demonstrations. A small dependency-free CLI exposes
diff, match, patch creation, and patch application while leaving the library API
primary. Files allow arbitrary bytes and avoid shell argument encoding limits.

## 18. Make `clippy::all` fatal; do not misrepresent `pedantic`

`make lint` treats rustfmt and every standard Clippy warning as errors. The
opt-in pedantic group is not globally fatal because its numeric-cast lints flag
the deliberate signed/unsigned coordinate translation and its length lints flag
large preserved table tests. Targeted standard warnings found during the audit
were fixed; hiding 177 pedantic messages behind blanket per-file allows would
provide less useful evidence.

## 19. Measure distributions and memory, not only hot-loop throughput

The benchmark driver records raw algorithm latency, fresh-process startup,
throughput, and peak RSS on identical source fixtures. It validates output shape
and records hashes/toolchains/confounders. Go statement coverage and Rust LLVM
coverage remain separately labeled because pretending their models are equal
would overstate precision.

## 20. Treat concurrency as immutability, not translated goroutines

The source has no goroutines or mutable global state. Rust configuration is
`Send + Sync`, algorithms borrow it immutably, and an eight-thread repeated
diff/patch test verifies reconstruction and application. Adding internal threads
would change performance and deadline behavior without source functionality to
justify it.

## 21. Preserve the source's right-boundary blank-line scoring typo

Upstream declares both `blanklineEndRegex` and `blanklineStartRegex`, but
`diffCleanupSemanticScore` applies the end expression to both sides and never
uses the start expression. The first full comparison campaign exposed the typo
through a minimized Unicode/CRLF patch whose application result matched but
whose serialized equality boundaries differed. Rust intentionally mirrors the
source line, despite the canonical algorithm's apparent intent, and a focused
regression locks down the observed v1.4.0 output. The reproducer and a separate
ready-to-file upstream report live under
`bug-cases/semantic-lossless-blankline-start/`.
