# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

## Build-time configuration axes

| source | axes found | valid combinations |
|--------|-----------|--------------------|
| `Cargo.toml` `[features]` | `default = []` — no optional features exist | **1**: the empty set (`--no-default-features`, which is identical to the default build) |
| `c_src/CMakeLists.txt` | no `option()`, no `add_definitions`, no `target_compile_definitions`, no generator/config branches | **1** |
| `c_src/src/main.c` | `grep -c '#if\|#ifdef\|#ifndef\|#else\|#elif' c_src/src/main.c` → **0** | **1** |

So the full build matrix is a single cell.  `run_all.sh` still loops over the
enumerated combination list (`""` = `--no-default-features`, and the default
build) so that Phases B and C are executed for **every** combination.

## Runtime configuration axes (derived from the C branches)

`c_src/src/main.c` has exactly one runtime branch and no global/persistent
state:

| axis | where | states |
|------|-------|--------|
| `line` NULL-ness | `printLine` line 28: `if (line != NULL)` | NULL → no output; non-NULL → `"%s\n"` |
| `line` byte content / length | argument of `printf("%s\n", line)` | empty, 1 byte, any of the 255 non-NUL byte values, ASCII, printf directives as data, embedded `\n`/`\r`/`\t`, invalid UTF-8, valid multi-byte UTF-8, lengths straddling `BUFSIZ` (4096/8192) and Rust `LineWriter` (1024) buffer sizes |
| `intNumber` value | argument of `printf("%d\n", intNumber)` | 0, ±1, digit-count transitions, `INT_MIN`, `INT_MAX` (sign + width of the decimal conversion) |
| statement form inside `bad`/`good` | line 43 `intOne + intTwo;` (result **discarded** — reproduce, do not fix) vs line 51 `intSum = intOne + intTwo;` | `bad()` → `0\n0\n`; `good()` → `0\n2\n` |
| `argc` / `argv` | `main` parameters — never dereferenced | any values, all ignored |
| call sequence / stdout buffering | all four printers share `stdout` | single call, repeated calls, interleaved calls, full-program sequence |

## Public entry points

All five external symbols are exercised **directly** through the `.so`
(`printLine`, `printIntLine`, `bad`, `good`, `main`) — the low-level printers are
not tested only via the `main` convenience path — plus the process-level
end-to-end comparison of the C executable against the Rust executable.

## Row table

Each row is run against BOTH `.so`s through `libloading` and compared
byte-for-byte; rows marked *(randomized)* use many property-style inputs with a
fixed seed (`tests/common/mod.rs::Rng`, seed `0x2026_0818_C0FF_EE01`).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `printLine` | `line = NULL` → the false side of the only branch (no output) | [x] |
| 2 | `printLine` | `line = ""` (zero-length, immediate NUL) | [x] |
| 3 | `printLine` | every single-byte string `0x01..0xFF` (255 cases, exhaustive) | [x] |
| 4 | `printLine` | random printable-ASCII strings, random lengths 1..256 *(randomized)* | [x] |
| 5 | `printLine` | random arbitrary non-NUL byte strings `0x01..0xFF` (invalid UTF-8 included), random lengths 1..512 *(randomized)* | [x] |
| 6 | `printLine` | strings containing printf directives as data (`%s`, `%d`, `%n`, `%%`, `%1$s`), randomly embedded *(randomized)* | [x] |
| 7 | `printLine` | strings containing embedded `\n`, `\r`, `\t`, `\v`, `\f` (line-buffer flush interaction), randomly placed *(randomized)* | [x] |
| 8 | `printLine` | lengths straddling stdio/`LineWriter` buffer boundaries: 1023, 1024, 1025, 4095, 4096, 4097, 8191, 8192, 8193, 65535, 65536, 1 MiB | [x] |
| 9 | `printLine` | valid multi-byte UTF-8: 2-, 3-, 4-byte sequences, emoji, combining marks, BOM | [x] |
| 10 | `printIntLine` | boundary integers: `0`, `1`, `-1`, `INT_MAX`, `INT_MAX-1`, `INT_MIN`, `INT_MIN+1` | [x] |
| 11 | `printIntLine` | full-range random `i32` *(randomized, 4096 samples)* | [x] |
| 12 | `printIntLine` | digit-count transitions ±(9,10,99,100,999,1000,…,10^9) and random small ints *(randomized)* | [x] |
| 13 | `bad` | single call — the discarded-`intOne + intTwo` path (`0\n0\n`) | [x] |
| 14 | `good` | single call — the assigning path (`0\n2\n`) | [x] |
| 15 | `bad` + `good` | repeated and interleaved calls (no cross-call state; random order) *(randomized)* | [x] |
| 16 | `main` | `argc = 1`, valid `argv` (`["driver", NULL]`) — full program output **and** return value | [x] |
| 17 | `main` | `argc = 0`, `argv = NULL` — parameters ignored | [x] |
| 18 | `main` | `argc = 64`, `argv` with 64 real strings — parameters ignored | [x] |
| 19 | `main` | called twice back-to-back — output is exactly the single-call output doubled | [x] |
| 20 | all five | randomized interleaving of `printLine`/`printIntLine`/`bad`/`good`/`main` in one capture (ordering + shared-`stdout` buffering) *(randomized)* | [x] |
| 21 | executable | C `c_src/build/driver` vs Rust `target/release/driver`: stdout bytes, stderr bytes, exit status, no argv, empty stdin | [x] |
| 22 | executable | same, with extra argv entries and 4 KiB of stdin piped in (both must ignore them; stdin must stay unread) | [x] |

## Result

All 22 rows are implemented in `tests/phase_b_valid.rs` (`row01`…`row22`) and all
pass, in both the `debug` and `release` profiles and for both enumerated feature
combinations:

```
suite `Phase B — valid paths (CONFIGS.md)`: 22 passed, 0 failed, 0 skipped (13587 captured .so calls)
```

That is ~6,790 individual C-vs-Rust byte-for-byte comparisons per run
(exhaustive over all 255 single-byte strings; randomized with the fixed seed
`0x2026_0818_C0FF_EE01` elsewhere).  Reproduce everything with `./run_all.sh`.
