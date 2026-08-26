# CONFIGS.md — Configuration-surface table (Phase A) / valid-path tests (Phase B)

Derived mechanically from the C source and the public header, not from a guess
about which configurations "matter".

## Axes the C code actually branches on

The complete public API (from `c_src/include/driver.h`, cross-checked against
`nm -D` on the C `.so`) is a single entry point, which is simultaneously the
lowest-level and the only entry point — there is no convenience wrapper layer
to hide behind:

```c
void driver(int x);
```

Enumerating the axes from the body
`for (int i = 0, j = 0; i < x; i++, j += 2) printf("%d %d\n", i, j);`:

| axis | values the code actually distinguishes | source of the distinction |
|---|---|---|
| **A. runtime options / modes / flags** | *none* — no setters, no globals, no context struct, no `#ifdef` (grep: 0 hits for `if`/`switch`/`#if`) | the API has one by-value `int` and no state |
| **B. sign class of `x`** | `x <= 0` (loop never runs) vs `x >= 1` (loop runs `x` times) | the guard `i < x` |
| **C. iteration-count shape** | empty (0) / one (1) / few (2..9) / many (10..10^6) | `i < x` |
| **D. decimal width of `i`** | 1-digit (`i<10`), 2 (`<100`), 3, 4, 5, 6, 7 digits — each changes the byte length of the line | `printf("%d ...")` |
| **E. decimal width of `j = 2*i`** | crosses a width boundary at a *different* `i` than axis D (`j` reaches 10 at `i=5`, 100 at `i=50`, 1000 at `i=500`, ...) so the two widths are independent axes of the same line | `printf("... %d\n")` |
| **F. line-count / total-byte volume** | small (fits one stdio buffer, < 4 KiB) vs large (forces many `write(2)` flushes of the 4 KiB `stdout` buffer) | glibc `stdout` buffering around `printf` |
| **G. `fd 1` kind** | regular file (glibc fully-buffered) vs pipe (also fully buffered but flushed on a different schedule / partial writes) | glibc chooses the buffering mode from `fstat` of fd 1 |
| **H. invocation sequence in one process** | single call / repeated identical calls / C-then-Rust vs Rust-then-C interleaving on the shared process-wide C `stdout` | both implementations write through the *same* `printf` of the *same* `libc.so.6`, so stream state is shared |

There is no element-type, byte-order, format-selection or count-parameter axis:
the ABI has no buffers, no pointers and no enums.

## Rows — the pruned cross-product the C treats differently

Every row is driven through the `.so` exports of **both** implementations and
compared byte-for-byte. Rows marked *randomized* use many inputs from a fixed
seed (`0x243F_6A88_85A3_08D3`), not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `driver` | B:`x>=1`, C:one → `x = 1` (smallest accepting input; D:1-digit `i`, E:1-digit `j`, F:small, G:file, H:single) | `c1_single_iteration` | [x] |
| C2 | `driver` | C:few → every `x` in `2..=9` exhaustively (D:1-digit, E:1-digit then crosses to 2-digit at `i=5`) | `c2_few_iterations_exhaustive` | [x] |
| C3 | `driver` | E boundary in isolation → `x` in `5..=8`: `j` crosses 9→10 while `i` stays 1 digit (asymmetric widths on one line) | `c3_j_width_crosses_before_i` | [x] |
| C4 | `driver` | D boundary → `x` in `9..=12`: `i` crosses 9→10 (`j` is already 2 digits) | `c4_i_width_crosses_ten` | [x] |
| C5 | `driver` | D+E boundaries → `x` in `49..=52` and `99..=102`: `j` crosses 99→100 at `i=50`, `i` crosses 99→100 at `i=100` | `c5_hundred_boundaries` | [x] |
| C6 | `driver` | D+E boundaries at the next decade → `x` in `499..=502`, `999..=1002` (`j` crosses 1000 at `i=500`; `i` crosses 1000) | `c6_thousand_boundaries` | [x] |
| C7 | `driver` | D+E boundaries at 10^4/10^5 → `x` in `4999..=5001`, `9999..=10001`, `49999..=50001`, `99999..=100001` | `c7_ten_thousand_and_hundred_thousand_boundaries` | [x] |
| C8 | `driver` | C:many, F:small, *randomized* → 300 random `x` in `1..=200` (G:file, H:single) | `c8_random_small` | [x] |
| C9 | `driver` | C:many, F:crosses the 4 KiB stdio buffer, *randomized* → 120 random `x` in `200..=5_000` | `c9_random_medium` | [x] |
| C10 | `driver` | C:many, F:many buffer flushes, *randomized* → 24 random `x` in `5_000..=120_000` (D up to 6 digits, E up to 6 digits) | `c10_random_large` | [x] |
| C11 | `driver` | F:very large volume → `x = 1_000_000` (~13 MB, 7-digit `i` and `j`; exercises the full width ladder in one call) | `c11_one_million` | [x] |
| C12 | `driver` | G:pipe instead of regular file, small + buffer-crossing sizes, *randomized* → 40 random `x` in `1..=3_000` written to a `pipe(2)` | `c12_random_over_pipe` | [x] |
| C13 | `driver` | H:repeated calls in one capture → `driver(x)` called 5× back-to-back for randomized `x` (concatenated output must match) | `c13_repeated_calls_concatenate` | [x] |
| C14 | `driver` | H:mixed sequence in one capture → random sequence of randomized `x` values (incl. rejecting `x<=0` mixed with accepting ones) run as one script against each library | `c14_mixed_call_script` | [x] |
| C15 | `driver` | H:cross-implementation interleaving → C call, then Rust call, then C call … within a single capture on the shared `stdout`; compared against the pure-C sequence | `c15_interleaved_c_and_rust` | [x] |
| C16 | `driver` | B:`x>=1` sweep over every power-of-two-ish shape → `x` in {1,2,3,4,7,8,15,16,31,32,63,64,127,128,255,256,511,512,1023,1024,2047,2048,4095,4096,8191,8192,65535,65536} | `c16_power_of_two_shapes` | [x] |

## Infeasible-but-documented configuration

| configuration | why not executed |
|---|---|
| `x` large enough that `j = 2*i` overflows `INT_MAX` (needs `i >= 2^30`) | 2^30 iterations × ~22 bytes ≈ 23 GB of output per implementation and well over 600 s of runtime. The Rust mirrors the C's compiled two's-complement behaviour with `wrapping_add`, and the arithmetic path itself is covered up to 7-digit values by C11. |
| `x == INT_MAX` executed to completion | same reason (2^31−1 iterations). Boundary handling of extreme `int` values is covered by `ERRORS.md` E4/E5/E7. |

## Phase B completion

All 16 rows pass byte-for-byte across their randomized inputs under the only
valid feature combination (the empty set == default; see `SYMBOLS.md`).
