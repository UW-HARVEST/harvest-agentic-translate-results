# CONFIGS.md — configuration surface table (Phase B)

Mechanically derived from `c_src/include/sieve.h` (the entire public API) and
every branch in `c_src/src/sieve.c`.

## Axes the C actually branches on

Enumerated from the source, not guessed:

1. **Public entry points.** `sieve.h` declares exactly one:
   `void sieve(int start)`. There is no convenience wrapper and no lower-level
   variant to reach past — `sieve` *is* the lowest level. `nm -D` on the C
   `.so` confirms one exported symbol.
2. **Runtime options / modes / flags.** None. No setter, no global, no
   context struct, no environment read, no `#ifdef` in the compiled source
   (`CMakeLists.txt` passes no `-D`). The only `#if` in the tree is the
   `SIEVE_H_` include guard.
3. **The one data-dependent branch:** `if (val % 10 == 9)`, i.e. the C
   truncating remainder of the argument. That partitions the input domain into
   the shapes below.
4. **Input shape axes** that follow from that branch plus the `int` width and
   the `val++`:
   * sign: positive / zero / negative;
   * residue `val % 10`: `== 9` (immediate stop) vs `!= 9` (iterate);
   * distance to the stopping value: 0, 1, few, many, crossing zero;
   * digit-width transitions, because `printf("%d")` changes the number of
     bytes it emits. Reading the C shows these can only happen mid-run for
     negative starts (`-100`→`-99`, `-1`→`0`) or across the overflow wrap: a
     positive run always terminates inside its own decade;
   * signed-overflow region (`val` near `INT_MAX`, where `val++` wraps);
   * `INT_MIN` / `INT_MAX` extremes.
5. **Output channel state**, since the observable result is entirely `stdout`
   bytes: buffered-to-file (fully buffered) vs unbuffered, and pre-existing
   unflushed buffer content shared between the C and Rust `.so`s.

## Rows

Each row is exercised in `translation/tests/valid_paths.rs` by loading **both**
`.so`s through `libloading` and comparing the captured `stdout` bytes
byte-for-byte. Rows marked *randomized* use a fixed-seed xorshift PRNG (seed
`0x5EED_1234_ABCD_EF01`) with many inputs per row, not one hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `sieve` | residue `== 9`, positive, single-iteration: `9`, `19`, `29`, `109`, `999999999`, `INT_MAX/10*10+9` | [x] |
| 2 | `sieve` | residue `== 9`, randomized positive multiples-of-10-plus-9 across the whole positive range | [x] |
| 3 | `sieve` | residue `== 0`, positive: `0`, `10`, `100`, `1000` — full 10-line run | [x] |
| 4 | `sieve` | residue `1..=8`, positive: every one of `1,2,…,8` and `11..=18` — partial runs of every length 2..=9 | [x] |
| 5 | `sieve` | randomized positive `val` in `1..=1_000_000` (residue uniform, run length 1..=10) | [x] |
| 6 | `sieve` | decade boundaries + mid-run `printf` width changes. Derived from the C, not assumed: a **positive** run can never change width, because it ends at the next value ending in 9, which is always inside the same decade (`95`->`99`, never `109`). Width changes mid-run only for **negative** starts (`-100`->`-99`, `-1`->`0`) and across the overflow wrap. Covers `5, 8, 95, 98, 995, 998, 9995, ..., 2147483635` and `-1, -10, -100, -1000, -105, -1005, -10005, -12` | [x] |
| 7 | `sieve` | negative, magnitude ending in 9 (`-9`, `-19`, `-129`): C truncating `%` yields `-9`, so **no** early stop — must count up to `+9` | [x] |
| 8 | `sieve` | negative, crossing zero: `-1`, `-5`, `-10`, `-11`, `-100`, `-123` — the run spans the sign change and the `-1`→`0` width change | [x] |
| 9 | `sieve` | randomized negative `val` in `-1_000_000..=-1` | [x] |
| 10 | `sieve` | randomized full-range `val` over all 2^32 bit patterns, filtered to runs short enough to compare in full (run length ≤ ~10 lines) | [x] |
| 11 | `sieve` | extreme low: `INT_MIN`, `INT_MIN+1`, `INT_MIN+8` — bounded output-prefix comparison from a forked child (full run is ~2.1e9 lines) | [x] |
| 12 | `sieve` | signed-overflow region: `INT_MAX`, `INT_MAX-1`, `INT_MAX-2` — `val++` wraps `INT_MAX`→`INT_MIN`; bounded output-prefix comparison, verifies the wrap sequence `2147483647`, `-2147483648`, … | [x] |
| 13 | `sieve` | output channel: `stdout` redirected to a regular file (fully buffered, default) — the mode all other rows run in | [x] |
| 14 | `sieve` | output channel: `stdout` set unbuffered via `setvbuf(_IONBF)` before the call, so each `printf` is its own `write(2)` | [x] |
| 15 | `sieve` | output channel: `stdout` set line-buffered via `setvbuf(_IOLBF)` | [x] |
| 16 | `sieve` | shared-buffer interaction: unflushed bytes written by the *test* (via libc `printf`) already sit in the `stdout` buffer when `sieve` is called; asserts both `.so`s append to the same FILE buffer rather than opening their own stream, so ordering is preserved | [x] |
| 17 | `sieve` | repeated / stateless invocation: the same `.so` handle called 200 times with randomized inputs in sequence; asserts no hidden state accumulates between calls (C has no globals — must stay true in Rust) | [x] |
| 18 | `sieve` | ABI width: the symbol invoked through an `extern "C" fn(i64)` signature with garbage in the upper 32 bits; both must read only the low 32 bits | [x] |
| 19 | `sieve` | interleaved C/Rust calls in one process (C, then Rust, then C…) on the same `stdout`, asserting the Rust `.so` does not disturb the C `.so`'s stream state | [x] |
