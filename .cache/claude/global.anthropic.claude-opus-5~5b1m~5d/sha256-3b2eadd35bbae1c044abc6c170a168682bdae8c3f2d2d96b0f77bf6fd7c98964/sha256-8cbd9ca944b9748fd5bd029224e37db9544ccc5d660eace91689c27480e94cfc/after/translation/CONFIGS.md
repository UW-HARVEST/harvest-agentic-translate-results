# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the C source. The whole public API is:

```c
void driver(int x, int y);          /* include/driver.h  — the ONLY entry point */
```

body (after resolving the ISO 646 spellings `bitor` = `|`, `compl` = `~`):

```c
int result = x | ~y;
printf("%d", result);   /* no newline */
puts("");               /* newline only */
```

## Axes the C code actually branches on

There are **no** runtime options, modes, flags, `#ifdef`s (other than the
header's include guard), `if`s, `switch`es, or `?:` in the C source, and no
mutable library state. Grep confirms:

```
grep -nE '#if|#ifdef|#else|if *\(|switch|\?|extern|static|global' c_src/src/driver.c c_src/include/driver.h
```
=> only `%:ifndef DRIVER_H_` / `%:define` / `%:endif` (include guard) and
`%:include` lines.

So the configuration surface is entirely the **input shape** of the two
`int` arguments, plus the shape of the *derived* value `result = x | ~y`
whose decimal formatting is what `printf("%d", …)` branches on internally
(sign, digit count). The axes are therefore:

* **A1 — sign of `x`**: negative / zero / positive.
* **A2 — sign of `y`**: negative / zero / positive.
* **A3 — magnitude class of each operand**: `0`, `±1`, small (< 10),
  multi-digit, near-`INT_MAX`, exactly `INT_MAX`, exactly `INT_MIN`.
* **A4 — sign of the printed `result`** (`x | ~y`): negative (a `-` is
  emitted) vs non-negative. Note `x | ~y` is non-negative **iff** `x >= 0`
  and `y < 0`, so this axis is *not* independent of A1/A2 and must be
  crossed with them.
* **A5 — decimal width of `result`**: 1 digit … 10 digits (+ optional `-`),
  i.e. the number of characters `printf` must emit — 1 through 11 bytes.
* **A6 — bit-pattern shape**: all-zero bits, all-one bits, single bit set,
  single bit clear, alternating (`0x55555555` / `0xAAAAAAAA`), sign bit only,
  arbitrary random pattern. These select different `~` / `|` results and
  different formatting lengths.
* **A7 — call multiplicity / statefulness**: one call vs many consecutive
  calls into the *same* loaded library (the two `stdio` calls per invocation
  mean output of consecutive calls must concatenate identically; a
  buffering/flush difference between C and Rust is only visible here).
* **A8 — entry-point level**: there is exactly one entry point and it is the
  lowest level one; there is no convenience wrapper vs. low-level split.
  `driver` is called directly via `dlsym` from both `.so`s in every row.

Every row below is exercised by loading BOTH `.so`s with `libloading`,
calling the exported `driver` symbol, capturing raw file-descriptor 1, and
comparing the captured bytes byte-for-byte. Rows marked "randomized" use
many pseudo-random inputs from a fixed-seed SplitMix64 generator.

## Configuration table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | A1=0, A2=0: `(0, 0)` — the minimal call; `result = ~0 = -1` | [x] |
| 2 | `driver` | A1>0, A2>0, both small single-digit: full cross product of `x,y ∈ 0..=9` (100 combos) | [x] |
| 3 | `driver` | A1<0, A2<0, both small negative: full cross product of `x,y ∈ -9..=0` | [x] |
| 4 | `driver` | mixed signs, small: full cross product `x,y ∈ -9..=9` (361 combos) | [x] |
| 5 | `driver` | A4 non-negative result branch (`x >= 0 && y < 0`), randomized: `x ∈ [0, INT_MAX]`, `y ∈ [INT_MIN, -1]` — printed value has **no** `-` sign | [x] |
| 6 | `driver` | A4 negative result branch (`x < 0` or `y >= 0`), randomized over each of the three sub-cases (`x<0,y<0`), (`x<0,y>=0`), (`x>=0,y>=0`) | [x] |
| 7 | `driver` | A5 = every decimal width 1..=10 digits of a non-negative `result`, constructed exactly (`result` = 0, 9, 99, …, 2147483647) via `x = result, y = INT_MIN` | [x] |
| 8 | `driver` | A5 = every decimal width of a negative `result` incl. the 11-byte worst case `-2147483648` (`x = INT_MIN, y = INT_MAX`) | [x] |
| 9 | `driver` | A3 boundary operands: full cross product of `{INT_MIN, INT_MIN+1, -65537, -256, -2, -1, 0, 1, 2, 255, 65536, INT_MAX-1, INT_MAX}` (169 combos) | [x] |
| 10 | `driver` | A6 bit-pattern shapes: cross product of `{0x00000000, 0xFFFFFFFF, 0x80000000, 0x7FFFFFFF, 0x55555555, 0xAAAAAAAA, 0x00000001, 0xFFFFFFFE}` reinterpreted as `int` (64 combos) | [x] |
| 11 | `driver` | A6 single-bit-set `x` × single-bit-clear `y`: `x = 1<<i`, `y = !(1<<j)` for all `i, j ∈ 0..32` (1024 combos) | [x] |
| 12 | `driver` | A1–A6 fully randomized: 20 000 uniformly random 32-bit `(x, y)` pairs, fixed seed | [x] |
| 13 | `driver` | A7 many consecutive calls, single capture: 500 randomized calls in one redirected-stdout window — the concatenated stream must match byte-for-byte (catches newline/flush/buffering divergence) | [x] |
| 14 | `driver` | A7 interleaved C-then-Rust and Rust-then-C ordering within one capture window, to rule out order-dependent buffering state | [x] |
| 15 | `driver` | A5 exhaustive-per-length sweep: for each output byte length 1..=11, 200 randomized inputs producing that exact length | [x] |
| 16 | `driver` | no-op / repeat determinism: the same input called twice must produce two identical copies of the same bytes in both libraries | [x] |

No binary executable is produced by either build (`c_src/CMakeLists.txt`
declares only `add_library(driver SHARED …)`; `translation/Cargo.toml`
declares only `[lib] crate-type = ["cdylib"]`), so the "compare the C and
Rust driver binaries' stdout" gate is not applicable; the equivalent
end-to-end stdout comparison is performed through rows 13–16, which capture
raw fd 1 exactly as a binary's stdout would be captured.
