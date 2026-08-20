# CONFIGS.md — configuration-surface table

## Build configurations

| axis | values | source of truth |
|---|---|---|
| CMake options / `#ifdef` | **none** | `c_src/CMakeLists.txt` has only `add_executable(driver src/main.c)`; `grep -c '#if\|#ifdef\|#ifndef' c_src/src/main.c` → 0 |
| cargo features | **none** (`[features]` is empty) | `Cargo.toml` |

So the complete set of valid feature combinations is the single empty
combination. `cargo check --no-default-features` and
`cargo check --all-features` are therefore the same build, and both are run by
`./check_all_configs.sh`, together with the `ffi` workspace member and the
`debug`/`release` profiles (the release profile differs materially: it sets
`panic = "abort"` and enables optimisation, which is what makes signed-overflow
wrapping and float formatting worth re-checking).

| # | combination | verified |
|---|---|---|
| F1 | `--no-default-features` (= the only combination), `cargo check` on the whole workspace | [x] |
| F2 | `--all-features` (identical to F1, no features exist) | [x] |
| F3 | default features, `--release` (`panic = "abort"`, optimised) | [x] |
| F4 | default features, `debug` (overflow checks **on** — proves nothing overflows outside `wrapping_*`) | [x] |

## Runtime configuration surface

`int main()` takes no arguments, reads no environment variable, calls no
`setlocale`, and opens no file. The C code's *entire* runtime input is:

1. the byte stream on **stdin**, and
2. the kind of file descriptor **stdout** is,

plus the FFI entry point `run(int)`, whose input is the `int` argument and the
accumulated state of the file-scope `the_house`.

### Axes the C actually branches on

| axis | branch site in `main.c` | distinct values |
|---|---|---|
| A. stdin readability | `fgets` (line 78) return ignored | readable / EOF-immediately / read error |
| B. terminator | `fgets` stops after `'\n'` (line 78) | `'\n'`-terminated / EOF-terminated / truncated at 99 bytes |
| C. length vs `sizeof(in)-1 == 99` | `fgets` (line 78) | 0, 1, 2…98, exactly 99, 100+ (cut before / inside / after the digits) |
| D. NUL in the buffer | `in` is a C string (line 77) | none / at offset 0 / inside the digits / after the digits |
| E. leading whitespace | `strtol` skips `isspace` (line 67) | none / `' '` / `'\t'` / `'\n'` / `'\v'` / `'\f'` / `'\r'` / mixtures / whitespace-only |
| F. sign | `strtol` (line 67) | none / `'+'` / `'-'` / doubled / sign-then-non-digit |
| G. digits | `strtol` base 10 (line 67) | none / 1 / many / leading zeros / 19 digits / 20+ digits |
| H. trailing bytes after the digits | `endp` is discarded (line 68) | none / `'\n'` / letters / `'.'` / more digits after a space |
| I. non-decimal bytes | `strtol` base **10**, so `'8'`,`'9'` are digits but `'x'`,`'a'`..`'f'` are not | `0x…` / `0b…` / `1e5` / high bytes `≥ 0x80` (never `isspace` in the "C" locale) |
| J. value class vs `errno`/`INT_MIN`/`INT_MAX` | line 68 | `0`, `±1`, small, `INT_MAX`, `INT_MIN`, `INT_MAX±1`, `INT_MIN−1`, `LONG_MAX`, `LONG_MIN`, `LONG_MAX+1`, `LONG_MIN−1`, ≫`LONG_MAX` |
| K. `bedrooms` arithmetic | `add_bedrooms` (line 43), called twice via two `run()`s | no overflow / single-`run` overflow / second-`run` overflow / `INT_MAX` / `INT_MIN` |
| L. stdout kind | `printf` (line 51, 84) | regular file / pipe / pipe with closed read end / closed fd 1 |
| M. entry point | — | process `main` / `run(int)` called directly through the `.so` |
| N. `run` call depth (state accumulation) | `the_house` is file-scope (line 36) | 1 call / 2 calls (what `main` does) / N≫2 calls, driving `floors`, `bedrooms`, `bathrooms` forward |

Axes A–L are all reachable *only* through stdin/stdout, which is why the
process-level differential test is the low-level entry point here: it is the
only way to exercise the `static` functions `parse_val`, `print_the_house`,
`add_floor`, `add_floor_to_the_house` and `add_bedrooms` at all. Axes M–N are
exercised additionally through `dlopen`-ing both `.so`s and calling the exported
`run` directly, which reaches call depths and `int` arguments that `main` can
never produce.

### Combination rows

Every row is checked with **many randomized inputs** (a fixed-seed xorshift64\*
generator, so the corpus is reproducible) unless it is a pure boundary that has
only one shape. All rows assert byte-identical stdout, byte-identical stderr and
identical exit status (code *and* signal) between the C and the Rust artifact.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | process `main` (stdin=file) | plain small non-negative value, `'\n'`-terminated (A=readable, B=newline, J=small) | [x] |
| C2 | process `main` | plain small negative value, `'\n'`-terminated | [x] |
| C3 | process `main` | value with **no** trailing newline (B=EOF-terminated) | [x] |
| C4 | process `main` | `'+'`-signed value (F=`+`) | [x] |
| C5 | process `main` | leading-zero runs of every length 1…40 before the digits (G=leading zeros) | [x] |
| C6 | process `main` | each single whitespace char from `{' ','\t','\v','\f','\r'}` as prefix (E) | [x] |
| C7 | process `main` | random mixtures of 0…20 whitespace chars, then sign, then digits (E×F×G) | [x] |
| C8 | process `main` | leading `'\n'` — `fgets` stops there, so the value after it is never seen (E×B) | [x] |
| C9 | process `main` | whitespace-only line (E, no digits) | [x] |
| C10 | process `main` | no-digit garbage: letters, punctuation, `'.'`, `'/'`, `':'` (G=none) | [x] |
| C11 | process `main` | sign followed by non-digit (F×G) | [x] |
| C12 | process `main` | trailing garbage after the digits: letters, `'.'`, `' '` + more digits, `0x…`, `1e5` (H×I) | [x] |
| C13 | process `main` | exact 32-bit boundaries `0, ±1, INT_MAX, INT_MIN, INT_MAX±1, INT_MIN±1` (J) | [x] |
| C14 | process `main` | exact 64-bit boundaries `LONG_MAX, LONG_MIN, LONG_MAX+1, LONG_MIN−1` (J) | [x] |
| C15 | process `main` | 20…98 digit values, i.e. far past `LONG_MAX` ⇒ `ERANGE` (J) | [x] |
| C16 | process `main` | uniformly random `i32` values, ±1 around them (J×K) | [x] |
| C17 | process `main` | uniformly random `i64` values rendered in decimal (J) | [x] |
| C18 | process `main` | line length swept over 0,1,2,…,101,102 with the value straddling the 99-byte cut (C×B) | [x] |
| C19 | process `main` | 100+-byte line whose 99-byte prefix is a *different valid* value (C13→C, e.g. 98 spaces + `"42"` ⇒ `4`) | [x] |
| C20 | process `main` | 100+-byte line whose 99-byte prefix becomes `ERANGE` (C×J) | [x] |
| C21 | process `main` | embedded NUL at offset 0 / inside the digits / right after the digits / after the newline (D) | [x] |
| C22 | process `main` | bytes `≥ 0x80` as prefix, as suffix, and interleaved; invalid UTF-8 (I) | [x] |
| C23 | process `main` | fully random byte strings, length 0…200, all 256 byte values (A×B×C×D×E×F×G×H×I) | [x] |
| C24 | process `main` | random *printable* strings biased towards digits/signs/spaces (grammar-directed fuzz) | [x] |
| C25 | process `main` | `extra_bedrooms` that overflows `bedrooms` on the **first** `run` (`INT_MAX`, `INT_MAX-4`, `2147483643`) (K) | [x] |
| C26 | process `main` | `extra_bedrooms` that overflows only on the **second** `run` (K) | [x] |
| C27 | process `main` | `extra_bedrooms = INT_MIN` (K, negative wraparound) | [x] |
| C28 | process `main` (stdin=EOF) | empty stdin / `/dev/null` (A=EOF) | [x] |
| C29 | process `main` (stdin=pipe) | value delivered through a pipe rather than a file (A, different `read` chunking) | [x] |
| C30 | process `main` (stdin=pipe, slow) | value split across multiple `read`s with the newline in a later chunk (A×B) | [x] |
| C31 | process `main` (stdin unreadable) | fd 0 closed, and fd 0 on a directory (A=read error) | [x] |
| C32 | process `main` (stdout=file) | stdout redirected to a regular file (L, full buffering) | [x] |
| C33 | process `main` (stdout=pipe) | stdout to a live pipe that is fully drained (L, full buffering) | [x] |
| C34 | process `main` (stdout=closed pipe) | stdout to a pipe with the read end closed (L ⇒ `SIGPIPE`) | [x] |
| C35 | process `main` (stdout closed) | fd 1 closed before `exec` (L ⇒ `EBADF`, ignored) | [x] |
| C36 | process `main` + argv | extra command-line arguments present (ignored by `int main()`) | [x] |
| C37 | `.so` `run(int)` fresh load | one call, `extra_bedrooms` ∈ boundary set `{0,±1,INT_MAX,INT_MIN,INT_MAX-4,…}` (M×N=1) | [x] |
| C38 | `.so` `run(int)` fresh load | two calls with the same argument — reproduces exactly what `main` does (M×N=2) | [x] |
| C39 | `.so` `run(int)` single load | 2000 calls with random `i32` arguments, comparing the whole accumulated transcript (M×N≫2, drives `floors`, `bedrooms` wraparound and `bathrooms` growth) | [x] |
| C40 | `.so` `run(int)` single load | 400 calls with arguments drawn from the boundary set, in random order (M×N×K) | [x] |
| C41 | `.so` `main()` | the exported `main` symbol of both `.so`s is the same function the executables run (covered end-to-end by C1–C36) | [x] |

## Row → test mapping

| rows | test | file |
|---|---|---|
| C1 – C36 | `cfg_c1_*` … `cfg_c36_argv_ignored` (one `#[test]` per row) | `tests/differential_process.rs` |
| C37 – C41 | `ffi_run_differential` (one `#[test]`, one section per row — see the file header for why the capture cannot be split across parallel tests) | `tests/ffi_capture.rs` |
| F1 – F4 | `./check_all_configs.sh --tests` (enumerates the feature power set from `Cargo.toml`, then `cargo check`/`build`/`test` in both profiles) | — |
| symbol parity | `symbol_parity_shared_objects`, `symbol_parity_executables`, `shared_objects_have_no_unresolved_symbols`, `cargo_built_cdylib_exports_the_same_symbols`, `no_stubbed_exports_in_the_translation` | `tests/symbols.rs` |
| gcc `-O` independence | `c_optimization_levels_agree_with_rust` (`""`, `-O0`, `-O1`, `-O2`, `-O3`, `-Os`) | `tests/differential_process.rs` |

Every row compares **stdout bytes, stderr bytes, exit code and terminating
signal**. The C artifacts are compiled out of tree into `target/difftest/` with
`cc`; nothing under `c_src/` is modified.

## Results

```
$ ./check_all_configs.sh --tests
features declared in Cargo.toml: 0 (none)
feature combinations to verify: 1
    OK: cargo check --no-default-features --features '' (workspace)
    OK: cargo check --release --no-default-features --features ''
    OK: cargo check --all-features (workspace, all targets)
    OK: cargo build (debug, workspace)
    OK: cargo build (release, workspace)
    OK: zero-warning build of every target (debug)     # RUSTFLAGS=-Dwarnings
    OK: zero-warning build of every target (release)   # RUSTFLAGS=-Dwarnings
    OK: cargo test --no-default-features --features '<none>' (debug)
    OK: cargo test --release --no-default-features --features '<none>'
    OK: cargo test --all-features (debug)
RESULT: all configurations OK
```

Per-target counts (identical in the `debug` and `release` profiles):

```
tests/differential_process.rs   37 passed
tests/error_paths.rs            20 passed
tests/ffi_capture.rs             1 passed   (all C37–C41 rows)
tests/symbols.rs                 5 passed
```

The whole suite was run 10 times back to back with 0 failures to confirm it is
not order- or timing-dependent. Beyond the suite, an out-of-band fuzz compared
**6 498** additional inputs (structured boundaries, length sweeps 0–110, 3 000
uniformly random byte strings up to 220 bytes, and 3 000 grammar-directed
strings) against the binary produced by `cmake --build` itself: 0 divergences.
