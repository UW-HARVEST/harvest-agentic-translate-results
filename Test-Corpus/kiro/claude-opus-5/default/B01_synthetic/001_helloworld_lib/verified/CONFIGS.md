# CONFIGS.md — Phase B configuration-surface table

The mirror of `ERRORS.md`, for VALID inputs. Axes are derived from what the C
actually branches on, using the same greps.

## Axis enumeration (mechanical)

| candidate axis | present in C? | evidence |
|----------------|---------------|----------|
| runtime options / modes / flags settable via the public API | **none** | `c_src/include/hello.h` declares exactly one function, `int helloworld();`. No setters, no globals, no context struct, no flags parameter. |
| compile-time options | **none** | the only preprocessor conditional in the tree is the `#ifndef HELLO_H_` include guard |
| input shapes (sizes / widths / element types / counts / formats / byte order / empty-one-many / boundary values) | **none** | the function takes **no parameters**, so it has no input to shape |
| `if` / `switch` branches on any flag | **none** | `grep -rEn "\bif\b|\bswitch\b|\bwhile\b|\bfor\b|\?"` over the whole C tree → 0 hits |
| public entry points, including lowest-level ones | **1** | `helloworld` is *both* the highest- and lowest-level entry point; there is no convenience wrapper layered over a lower-level API to skip |

So the cross-product of API-level option axes is a single point. The C library
genuinely has one entry point, no arguments and no branches.

That does **not** mean one test suffices. `helloworld`'s observable output is not
just its return value — it is the bytes it puts into the process's libc `stdout`
stream. The axes the *behaviour* actually varies over, and which a wrong
translation would get wrong (notably one using `std::io::stdout` instead of libc
`printf`/`puts` — see `SYMBOLS.md` §2), are the **stdout destination, buffering
mode, interleaving and concurrency** axes below. Each row is a combination that
is treated differently by the underlying stdio machinery the C depends on.

Every row: load BOTH `.so`s via `libloading`, call the exported `helloworld`
symbol in that configuration, capture fd 1 at the file-descriptor level, and
assert the captured bytes and return values are byte-for-byte identical between
C and Rust. Rows marked *randomized* draw their call counts / interleaving
patterns / chunk sizes from a fixed-seed PRNG (seed `0x5DEECE66D`, 64 cases per
row unless noted) rather than one hand-picked value.

## The table

| # | entry point(s) | configuration (options set + input shape) | randomized | ✔ |
|---|----------------|------------------------------------------|-----------|---|
| C1 | `helloworld` | Single call, stdout → regular file (fully buffered). Baseline: exact bytes `"Hello World!\n"` (13 bytes) and return `0`. | no | [x] |
| C2 | `helloworld` | `N` successive calls, stdout → regular file, `N` drawn from the PRNG over `1..=512`. Covers empty/one/many. Output must be `N` copies of the line. | yes | [x] |
| C3 | `helloworld` | Call count chosen to straddle the **stdio buffer boundary**: `N` such that `13*N` lands just below / at / just above 1024, 4096 and 8192 bytes (the glibc `BUFSIZ`/page flush points). Boundary values for the only "size" the function has. | yes | [x] |
| C4 | `helloworld` | stdout → **pipe** (fully buffered, and the reader can observe flush timing) vs → **regular file** vs → **`/dev/null`** (character device). glibc picks the buffering mode from `fstat` on fd 1, so these are three genuinely different stdio paths. | yes | [x] |
| C5 | `helloworld` | **Interleaved with caller-side libc `printf`** writes: random alternation of `helloworld()` and the harness's own `printf` of a marker line. This is the row that catches a translation writing to a *different* buffer than libc's — ordering would scramble. | yes | [x] |
| C6 | `helloworld` | **Interleaved with caller-side raw `write(2)`** to fd 1, with an explicit `fflush(NULL)` between phases. Checks that flush points are where the C's are. | yes | [x] |
| C7 | `helloworld` | **Interleaved with `std::io::Write` on Rust's `stdout()`**, flushed at the boundaries — mixes the harness's Rust-side buffer with the library's libc buffer. C and Rust libraries must order identically relative to it. | yes | [x] |
| C8 | `helloworld` | **stdout set to unbuffered (`_IONBF`)** and to **line-buffered (`_IOLBF`)** via `setvbuf`, then `N` calls. Different stdio write paths (per-call `write` syscall vs per-line vs block). | yes | [x] |
| C9 | `helloworld` | **No explicit flush at all** — rely on the flush that `fflush(NULL)` performs at capture time, with `N` calls left sitting in the buffer. Confirms buffered-but-unflushed byte counts agree. | yes | [x] |
| C10 | `helloworld` | **Concurrent calls from `T` threads × `K` calls each** (`T` in `2..=8`, `K` in `1..=64`), stdout → file. glibc locks the FILE, so each line is atomic and — since every line is identical — the byte stream is deterministic. Checks the Rust export is thread-safe like the C. | yes | [x] |
| C11 | `helloworld` | **Both `.so`s loaded simultaneously** in one process, calls alternating between the C and Rust `helloworld` in a PRNG-chosen pattern. Verifies `RTLD_LOCAL` isolation (no symbol interposition) and that the two are interchangeable mid-stream: the combined output must equal the same total line count regardless of which library produced each line. | yes | [x] |
| C12 | `helloworld` | Return value under load: assert `0` on **every one** of the calls in rows C2–C11, not just the first — a value-dependent or call-count-dependent return would otherwise hide. | yes | [x] |
| C13 | `helloworld` | Symbol re-resolved fresh (`dlsym`) before each of `N` calls, vs resolved once and reused. Probes lazy-PLT-binding differences between the two objects. | yes | [x] |

All 13 rows pass across their randomized inputs (see `tests/differential.rs`).

## Feature combinations

None exist — `translation/Cargo.toml` has no `[features]`, no `optional`
dependencies, and the crate source contains no `cfg(feature)`. The table above is
therefore the complete configuration surface, and the default build is the only
build. See `SYMBOLS.md` §3.
