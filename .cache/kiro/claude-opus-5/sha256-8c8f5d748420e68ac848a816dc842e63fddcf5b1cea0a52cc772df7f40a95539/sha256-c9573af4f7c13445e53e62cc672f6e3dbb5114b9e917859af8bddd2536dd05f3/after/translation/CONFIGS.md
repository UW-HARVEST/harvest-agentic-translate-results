# CONFIGS.md — Configuration-surface table

The mirror of `ERRORS.md`: the **valid** input space. Derived from the axes the
C source actually branches on or is sensitive to, not from what looks
important.

## Axis derivation

**Runtime options / modes / flags.** Grepping the public header and the source
for settable state:

```sh
grep -nE 'if |switch|#if|extern|static [^v]|global|_set|flag|mode|option' src/driver.c include/driver.h
```

Result: **there are none.** The library has no configuration function, no
global variable, no `#ifdef`-selected behaviour, no init/teardown, and no
opaque context struct. `driver.h` declares a single symbol, `void driver(void)`.
The only `if` in the whole library is `printLine`'s null check. So the
option axis is a single point, and the configuration surface is spanned
entirely by the remaining two axes.

**Input shapes the code is sensitive to.** `printLine` is the only function
with a parameter (`const char *`). Its body is `printf("%s\n", line)`, so the
shapes that can produce different behaviour are: pointer nullity (→ `ERRORS.md`
row 1), string length (including the stdio buffer boundary, where flushing
behaviour changes), and byte content (`%` conversion specifiers, embedded
newlines, embedded high/non-UTF-8 bytes — the last of which a naive
`CStr::to_str()`-based translation would reject or mangle).

**Full set of public entry points, lowest-level first.** From `nm -D`:

| level | entry point | signature |
|-------|-------------|-----------|
| 0 (lowest — the output primitive) | `printLine` | `void printLine(const char *)` |
| 1 (calls level 0 once)            | `bad`       | `void bad(void)` |
| 1 (calls level 0, then a private helper that calls level 0) | `good` | `void good(void)` |
| 2 (composes levels 0 and 1 — six calls in a fixed order) | `driver` | `void driver(void)` |

`printLine` is exported but **not** declared in `driver.h`; it is nonetheless a
real public entry point (an external caller can `dlsym` it, which is exactly
what these tests do) and it is the lowest-level one, so it is exercised
directly rather than only through `driver`. Rows 1–10 drive level 0 directly;
rows 11–13 drive levels 1–2; rows 14–16 cover cross-entry-point composition
and process-level output state.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | Single ASCII printable string, randomized length 1–64 and randomized content. 256 random cases. | [x] |
| 2 | `printLine` | String of length exactly 1, swept over **every** byte value `0x01`–`0xFF` (all 255 valid one-char strings). | [x] |
| 3 | `printLine` | String containing arbitrary non-NUL bytes `0x01`–`0xFF` incl. high bytes / invalid UTF-8, randomized length 1–128. 256 random cases. Catches any `to_str()`/UTF-8-validating translation. | [x] |
| 4 | `printLine` | Boundary: length exactly `BUFSIZ-1` (4095), stdio-buffer edge. | [x] |
| 5 | `printLine` | Boundary: length exactly `BUFSIZ` (4096) — the emitted line is 4097 bytes and straddles the buffer. | [x] |
| 6 | `printLine` | Boundary: length exactly `BUFSIZ+1` (4097). | [x] |
| 7 | `printLine` | Long string, randomized length 4000–4200 (sweeps the buffer boundary with random content). 64 random cases. | [x] |
| 8 | `printLine` | Very long string, 64 KiB, randomized content — many buffer flushes. | [x] |
| 9 | `printLine` | Oversized: 1 MiB randomized content. | [x] |
| 10 | `printLine` | Content special-cases the `printf` path could mishandle: embedded `\n`, `\t`, `\r`, `%s`, `%d`, `%n`, `%%`, `%1000000d`, lone `%` at end, backslashes, NUL-adjacent guard byte after the terminator. 22 hand-built + 256 randomized `%`-heavy cases. | [x] |
| 11 | `bad` | No arguments (only possible configuration). Single call. | [x] |
| 12 | `good` | No arguments. Single call — asserts the `good`→`helperGood` call is present **and** that `bad` does *not* emit `helperBad()`, i.e. the C's asymmetry is preserved. | [x] |
| 13 | `driver` | No arguments. Full end-to-end run: the exact 6-line sequence, in order. | [x] |
| 14 | `printLine`, `bad`, `good`, `driver` | Composition / interleaving: randomized sequences of 1–40 calls drawn from all four entry points (with randomized `printLine` payloads, incl. NULL), run as one uninterrupted stream against one captured fd. 128 random sequences. Catches ordering, buffering, and residual-state bugs invisible to per-function tests. | [x] |
| 15 | `driver` | Idempotence / no residual state: `driver` called 50 times in a row; output must equal 50 concatenated copies of the single-call output. | [x] |
| 16 | `printLine`, `driver` | Output fd is a **pipe** (glibc line-buffered) rather than a regular file (fully buffered) — the one process-level state that changes libc's flushing behaviour. | [x] |

All 16 rows verified byte-for-byte against the C `.so`.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section**:

```sh
$ grep -c '\[features\]' translation/Cargo.toml
0
```

Therefore the feature power-set is the single element `{}` (default = no
features), and `--no-default-features` is equivalent to the default build.
`scripts/check_all_features.sh` enumerates the feature list from
`Cargo.toml` and loops over the power-set; with zero features it runs the
default and `--no-default-features` configurations, both of which build and
pass the full suite. There is no second code path to cover.
