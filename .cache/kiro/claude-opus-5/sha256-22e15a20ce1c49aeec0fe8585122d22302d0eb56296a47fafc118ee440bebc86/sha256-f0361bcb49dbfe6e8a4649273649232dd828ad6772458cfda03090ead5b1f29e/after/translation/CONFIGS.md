# CONFIGS.md — Configuration-surface table

The mirror of `ERRORS.md`: every **valid** input configuration the C actually
distinguishes. Derived mechanically from the source, not from a guess about what
"matters".

## Mechanical derivation of the axes

### Axis 1 — public entry points (the FULL set, incl. the lowest level)

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001143 T driver
0000000000001119 T printHexCharLine
```

| entry point | level | declared in header? |
|---|---|---|
| `printHexCharLine(char)` | **lowest level** — does the formatting/emission itself | no (exported but undeclared) |
| `driver(char)` | convenience wrapper — computes `data + 1`, then calls `printHexCharLine` | yes |

`driver` is the one-shot wrapper; `printHexCharLine` is the primitive. Both are
exercised **directly** below, and the composition `driver → printHexCharLine` is
exercised as a pipeline (row C14), because a wrapper-only test cannot see a bug
in the increment and a primitive-only test cannot see a bug in the composition.

### Axis 2 — runtime options / modes / flags

```
$ grep -cE 'if|switch|\?|#ifdef|#if |global|static|extern [^v]' c_src/src/driver.c
0
```

**There are no runtime options, modes, flags, or global state.** The API sets
nothing and branches on nothing. There is no `set_*`/`init`/`config` function,
no environment variable read, no compile-time variant. Both functions are pure
straight-line code over their single argument.

The only *state* in play is the one both implementations share: the libc
`stdout` stream and its buffering mode. That is an environment axis rather than a
library option, and it is included as rows C15/C16 because output ordering and
flush timing are part of "byte-identical".

### Axis 3 — input shapes the code special-cases

The complete input domain of the whole library is a single `char`, i.e. **256
values per entry point** — small enough to enumerate exhaustively (rows C1/C2),
which is the strongest available check. Beyond exhaustiveness, these are the
distinct regimes the *code path* distinguishes, all of them consequences of two
things the C does: the default argument promotion of a **signed** `char` into the
variadic `printf`, and the `02` minimum-width flag:

| regime | boundary constant | why the code treats it differently |
|---|---|---|
| zero | `0x00` | both digits come from the `02` pad |
| one hex digit | `0x01`–`0x0F` | one digit from the value, one from the pad |
| two hex digits, positive | `0x10`–`0x7E` | pad flag inert |
| `CHAR_MAX` | `0x7F` | last non-sign-extended value; for `driver` also the input that overflows the truncating store |
| `CHAR_MIN` | `0x80` (`-128`) | first sign-extended value → 8 hex digits, pad flag inert |
| negative | `0x81`–`0xFE` | sign-extended to `0xFFFFFFxx` |
| `-1` | `0xFF` | maximal sign extension; for `driver`, the unique input yielding `0` |

### Axis 4 — FFI argument width (byte order / representation)

The parameter is `char`, narrower than the register that carries it. An external
caller can present the symbol with a full-width `int`; the SysV callee reads only
the low 8 bits. Both canonical (`i8`) and non-canonical (`c_int` with nonzero
upper bytes) call shapes are therefore real, distinct configurations (rows
C12/C13). Byte order is not an axis: there is no multi-byte value in the API.

### Axis 5 — call multiplicity

Single call, long sequence of calls from one library, and a sequence that
interleaves the C and Rust `.so`s on the same `stdout` (rows C14–C16). This
catches lazy-initialisation and caching differences a single call cannot.

## Configuration-surface table

One row per meaningful combination the C treats differently. Every row is driven
through **both** `.so`s' exported symbols via `libloading`, with many randomized
inputs (fixed seed `0x5EED_C0FFEE`, SplitMix64) except where the row is
exhaustive or a single named boundary. A row is checked only after it passes
across all its inputs.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `printHexCharLine` | no options (none exist) + **exhaustive** all 256 `char` values `0x00..0xFF`, canonical `i8` argument | [x] |
| C2 | `driver` | no options + **exhaustive** all 256 `char` values `0x00..0xFF`, canonical `i8` argument | [x] |
| C3 | `printHexCharLine` | value regime `zero`: `0x00` (both output digits produced by the `02` pad) | [x] |
| C4 | `printHexCharLine` | value regime `one hex digit`: randomized in `0x01..0x0F` (pad supplies the leading digit) | [x] |
| C5 | `printHexCharLine` | value regime `two digits, positive`: randomized in `0x10..0x7F` (pad flag inert, no sign extension) | [x] |
| C6 | `printHexCharLine` | value regime `negative`: randomized in `0x80..0xFF` (sign-extended to 8 hex digits, pad flag inert) | [x] |
| C7 | `driver` | input regime where the increment stays inside one hex digit: randomized in `0x00..0x0E` | [x] |
| C8 | `driver` | input regime where the increment produces two positive digits: randomized in `0x0F..0x7D` | [x] |
| C9 | `driver` | boundary `0x7E` and `0x7F`: the last input that does **not** overflow the truncating store, and the one that does (`127 + 1` → `-128`) | [x] |
| C10 | `driver` | input regime `negative, result negative`: randomized in `0x80..0xFD` (both operand and result sign-extended) | [x] |
| C11 | `driver` | boundary `0xFE`, `0xFF`: result `-1` (max sign extension) and result `0` (unique all-zero output) | [x] |
| C12 | `printHexCharLine` | FFI argument-width axis: symbol called through `void(*)(c_int)` with randomized **full 32-bit** values plus named `INT_MIN`, `INT_MAX`, `0x1FF`, `0xFFFFFF00` — no `char` representation, low byte sign-extended by the callee | [x] |
| C13 | `driver` | FFI argument-width axis: same as C12 for `driver` (randomized full 32-bit + named boundaries) | [x] |
| C14 | `printHexCharLine` **and** `driver` (composed pipeline) | cross-entry-point identity the composition must satisfy: for every one of the 256 inputs, `driver(x)` and `printHexCharLine((char)(x+1))` must emit the same bytes — within C, within Rust, and across C↔Rust | [x] |
| C15 | both, interleaved | call multiplicity: one randomized sequence of 4096 mixed `driver`/`printHexCharLine` calls with random values, emitted into a single capture, whole transcript compared C vs Rust byte-for-byte | [x] |
| C16 | both, interleaved on one shared `stdout` | buffering/ordering axis: C and Rust calls alternated into the *same* fully-buffered `stdout` with no intermediate flush, then flushed once — verifies both write through the same libc stream in the same order (a Rust translation using `std::io::stdout` would reorder here) | [x] |
| C17 | all of C1–C16 | build configuration axis: re-run under `--no-default-features` (the only other combination; `Cargo.toml` has no `[features]` section, so this is the complete cross-product of build configs) | [x] |

## Where each row is verified

Rows C1–C16 are one function each in `tests/phase_b_valid_paths.rs`, named
`c1_…` … `c16_…` and run sequentially from that binary's `main`. Row C17 (the
build-configuration axis) is driven by `scripts/verify.sh`, which re-runs the
whole of Phases B and C once per configuration.

```
$ cargo test --release
row c1_print_hex_char_line_exhaustive_all_256 ... ok
row c2_driver_exhaustive_all_256 ... ok
...
row c16_shared_stdout_interleaved_ordering ... ok

Phase B: all 16 rows passed
```

### Why the tests use `harness = false`

The library's only observable effect is bytes on file descriptor 1, so the
harness must `dup2` over fd 1 to capture output. fd 1 is process-global, and
libtest prints its own progress lines to it from the main thread while test
threads are still running — under the default parallel harness those lines land
inside the capture file and produce spurious divergences. Both phase binaries
therefore declare `harness = false` in `Cargo.toml` and run their rows
sequentially, which keeps per-row reporting without the interference.

### Non-vacuity (negative control)

A passing suite is only meaningful if it can fail. `scripts/verify.sh` builds a
deliberately mutated library (`%02X` instead of `%02x`, and `d + 2` instead of
`d + 1`), points the harness at it via `DRIVER_RUST_SO`, and requires the run to
**fail**. This is checked under every configuration:

```
[ok]   negative control: suite correctly rejects a mutated library
```
