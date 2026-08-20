# CONFIGS.md — Phase B: configuration-surface table

## Build-time configuration axes

* `Cargo.toml` has **no `[features]` section**, so there is exactly one Rust
  feature combination: the empty set.  `--no-default-features` and the default
  build are therefore the same configuration, and both are still exercised (see
  `run_all.sh`).
* `c_src/CMakeLists.txt` has **no `option()`, no `add_definitions`, no
  `target_compile_definitions`, no `#ifdef` in the source** — `C_DEFINES` and
  `C_INCLUDES` come out empty and `C_FLAGS = -fPIE`.  One C configuration.

```
$ grep -c 'features' Cargo.toml            # -> 0
$ grep -nE 'option|DEFIN|ifdef|ifndef' c_src/CMakeLists.txt c_src/src/main.c   # -> no output
```

## Runtime configuration axes (derived from the C source)

There are no flags/modes/options in the API.  The axes the C code actually
branches on are:

| axis | values the C distinguishes |
|------|-----------------------------|
| entry point | `fma_array` (lowest level), `driver` (calls `fma_array` with all four pointers aliased, then `printf`s), `main` (reads stdin, calls `driver`) |
| `int len` | `< 0` (`-1`, `INT_MIN`), `0`, `1`, `2..8`, `100` (the size `main` uses), `> 100` (`1000`), `> caller's buffer` |
| pointer aliasing of `out`/`mul1`/`mul2`/`add` | all distinct; `out==mul1`; `out==mul2`; `out==add`; `mul1==mul2`; all four equal (the shape `driver` uses); forward partial overlap; backward partial overlap |
| element values | zeros; ones; `-1`; small (no overflow of `x*y+z`); values straddling the `int` overflow boundary (`46340`/`46341`, `65536`); `INT_MIN`/`INT_MAX`; uniformly random over the whole `i32` range (overflow is the common case) |
| stdin token count (`main`) | `0`, `1`, `2..98`, `99`, `100`, `101..150` (past the capacity bound) |
| stdin separators (`main`) | single space; runs of mixed `isspace` bytes (` `, `\t`, `\n`, `\v`, `\f`, `\r`); leading whitespace; trailing newline present/absent; **no** separator (`"1-2+3"`) |
| stdin token spelling (`main`) | bare digits; `+`-signed; `-`-signed; leading zeros; magnitude classes small / near `INT_MAX`,`INT_MIN` / `(INT_MAX, LONG_MAX]` / `> LONG_MAX` / `< LONG_MIN` |

Every row below is driven through **both** `.so`s with `libloading` (or, for
`main`, through both `.so`s in a forked child *and* through both compiled
programs) and is run with many randomized inputs from a fixed-seed SplitMix64
PRNG — 200 iterations per row unless noted.  Outputs compared: the full `out`
buffer bytes, all captured stdout bytes, and the exit status / termination
signal.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `fma_array` | 4 distinct buffers, `len=1`, uniformly random `i32` values | [x] |
| 2  | `fma_array` | 4 distinct buffers, `len` random in `2..=8`, random values | [x] |
| 3  | `fma_array` | 4 distinct buffers, `len=100`, random values | [x] |
| 4  | `fma_array` | 4 distinct buffers, `len=1000`, random values | [x] |
| 5  | `fma_array` | 4 distinct buffers, `len` in `1..=64`, **small** values (`-100..=100`, no overflow) | [x] |
| 6  | `fma_array` | 4 distinct buffers, `len` in `1..=64`, values drawn from the extreme set `{0,±1,2,46340,46341,65535,65536,INT_MIN,INT_MAX,INT_MIN+1,INT_MAX-1}` | [x] |
| 7  | `fma_array` | 4 distinct buffers, uniform buffers (all `0` / all `1` / all `-1` / all `INT_MIN` / all `INT_MAX`), `len` in `1..=64` | [x] |
| 8  | `fma_array` | `out == mul1`, `mul2`/`add` distinct, random values, `len` in `1..=64` | [x] |
| 9  | `fma_array` | `out == mul2`, `mul1`/`add` distinct | [x] |
| 10 | `fma_array` | `out == add`, `mul1`/`mul2` distinct | [x] |
| 11 | `fma_array` | `mul1 == mul2` (squaring), `out`/`add` distinct | [x] |
| 12 | `fma_array` | `out == mul1 == mul2 == add` (exactly what `driver` does), random values, `len` in `1..=64` | [x] |
| 13 | `fma_array` | forward partial overlap: `out = base`, `mul1 = mul2 = add = base+1` | [x] |
| 14 | `fma_array` | backward partial overlap: `out = base+1`, `mul1 = mul2 = add = base` (stores clobber later loads) | [x] |
| 15 | `fma_array` | `len = 0` with four valid non-NULL buffers (must leave every byte untouched) | [x] |
| 16 | `fma_array` | `len = -1`, `-100`, `INT_MIN` with valid buffers | [x] |
| 17 | `driver` | `len = 0`, valid buffer → empty stdout, buffer untouched | [x] |
| 18 | `driver` | `len = -1` / `INT_MIN`, valid buffer → empty stdout, buffer untouched | [x] |
| 19 | `driver` | `len = 1`, uniformly random value (stdout **and** mutated buffer compared) | [x] |
| 20 | `driver` | `len` random in `2..=8`, random values | [x] |
| 21 | `driver` | `len = 100` (the size `main` uses), random values | [x] |
| 22 | `driver` | `len = 1000` (past what `main` can produce), random values | [x] |
| 23 | `driver` | `len` in `1..=64`, extreme value set (row 6's set) — exercises `printf("%d")` on `INT_MIN` etc. | [x] |
| 24 | `driver` | `len` in `1..=64`, small values (`-100..=100`, no overflow) | [x] |
| 25 | `driver` | called **twice** on the same buffer (`x → x²+x` applied twice), `len` in `1..=32`, random values | [x] |
| 26 | program + `main` export | empty stdin | [x] |
| 27 | program + `main` export | 1 token, uniformly random `i32`, single trailing `\n` | [x] |
| 28 | program + `main` export | `k` random in `2..=98` tokens, single-space separated, uniformly random `i32` | [x] |
| 29 | program + `main` export | exactly 99 and exactly 100 tokens | [x] |
| 30 | program + `main` export | `k` random in `101..=150` tokens (capacity bound: only the first 100 are read) | [x] |
| 31 | program + `main` export | tokens separated by random runs of mixed `isspace` bytes (` `,`\t`,`\n`,`\v`,`\f`,`\r`) | [x] |
| 32 | program + `main` export | random leading whitespace run, and no trailing newline at all | [x] |
| 33 | program + `main` export | random mix of sign spellings: bare, `+`, `-`, and random-length leading zero runs | [x] |
| 34 | program + `main` export | random mix of magnitude classes: small, near `INT_MAX`/`INT_MIN`, `(INT_MAX, LONG_MAX]`, `> LONG_MAX`, `< LONG_MIN` | [x] |
| 35 | program + `main` export | no separators — tokens glued by their signs (`"1-2+3-4"`), random count/values | [x] |
| 36 | program + `main` export | `k` random valid tokens followed by a random invalid token, then more valid ones | [x] |
| 37 | program + `main` export | fully random byte soup over the alphabet `{digits, +, -, space, \t, \n, \v, \f, \r, ., x, a, \0, 0x80, 0x7f}` (fuzz, 600 iterations) | [x] |
| 37b| program + `main` export | longer (60..500 byte) digit-biased byte soup, so many tokens convert and some digit runs saturate `strtol` (fuzz, 300 iterations) | [x] |
| 38 | `main` export | `main` called 1, 2 and 3 times **in the same process** with 1..260 tokens on stdin: C's `stdin` is a global `FILE`, so each call resumes the stream where the last one stopped (100 / 200 / 250 lines for 250 tokens) | [x] |
| 39 | `fma_array` | four disjoint windows at a **misaligned** base address (`int *` at byte offset 1): no alignment check anywhere in the C, just unaligned x86-64 loads | [x] |

Row/test mapping: rows 1-16 → `tests/fma_array.rs::row01..row16`, rows 17-25 →
`tests/driver_fn.rs::row17..row25` (out-of-process) **and**
`tests/in_process_stdout.rs` (same rows, library called in-process through
`libloading`), rows 26-39 → `tests/program.rs::row26..row39`.
