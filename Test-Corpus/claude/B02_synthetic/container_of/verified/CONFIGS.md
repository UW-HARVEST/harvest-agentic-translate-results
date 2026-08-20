# CONFIGS.md — configuration-surface table (Phase A → Phase B)

## Build-time configurations (enumerated mechanically)

```sh
grep -n '\[features\]' Cargo.toml   # -> no match: the crate declares no features
grep -nE 'option|if *\(|CMAKE_BUILD_TYPE|add_definitions|target_compile' \
     c_src/CMakeLists.txt           # -> no match: a single unconditional target
grep -nE '#if|#ifdef|#ifndef|#else|#elif' c_src/src/container_of.c
                                    # -> no match: no conditional compilation
```

* **Rust feature combinations: exactly one** — the empty set. `Cargo.toml` has no
  `[features]` table, so `--no-default-features`, `--all-features` and the plain
  default build are the *same* configuration. It is still exercised explicitly
  (see `./check_all_features.sh`) so the claim is verified rather than assumed.
* **C build configurations:** `CMakeLists.txt` declares one target with no
  options and the C source has no `#ifdef`, so there is one C configuration.
  Because the C code relies on behaviour the standard leaves undefined (signed
  overflow, out-of-bounds pointer arithmetic), the reference is additionally
  built at `-O2` and compared as well (rows 21–23), to prove the Rust matches the
  optimised code generation too, not just the default (`-O`-less) one.

## Runtime configuration axes the C code actually branches on

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| entry point | `find_container_of_a`, `find_container_of_b`, `main` | the three exported symbols |
| member offset | `offsetof(struct test, a) == 0` vs `offsetof(struct test, b) == 4` | `container_of` expansion |
| pointer value | any 64-bit value; `< 4` wraps below zero for `_b`; `NULL`; live-object addresses | unchecked `(char *)ptr - offset` |
| `atoi` lexical form | leading `isspace` run / optional `+`/`-` / digit run / first non-digit stops the scan / empty subject sequence | glibc `strtol(…, 10)` |
| `atoi` magnitude class | fits `int` · fits `long` only (truncated) · `> LONG_MAX` (saturates) · `< LONG_MIN` (saturates) | `strtol` + `(int)` cast |
| `printf("%d\n")` value class | positive · zero · negative · `INT_MIN` (the one value needing 11 chars) | format conversion |
| sum class | no overflow · overflow past `INT_MAX` · underflow past `INT_MIN` | unchecked `+` |
| `argc` | never read: 0, 1, 2, 3, many, negative, `INT_MAX` are all equivalent | `main` ignores its first parameter |
| `argv` length | ≥ 3 entries (prints) vs fewer (faults, see ERRORS.md) | unconditional `argv[1]`, `argv[2]` |
| stdout kind | file / pipe (fully buffered) vs terminal (line buffered) — affects *when*, never *what* | `printf` + implicit exit flush |

Every row below is checked with **many randomized inputs** (a fixed-seed
xorshift64\* PRNG lives in `tests/common/mod.rs`, so runs are reproducible), not a
single hand-picked value, and compares the C `.so` and the Rust `.so` byte for
byte through `libloading`.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `find_container_of_a` | 4096 uniformly random 64-bit pointer values (no dereference) | [x] |
| 2  | `find_container_of_a` | boundary addresses: `0`, `1`, `2`, `3`, `4`, `7`, `8`, `4095`, `4096`, `2^31`, `2^32`, `2^63`, `usize::MAX-3…MAX` | [x] |
| 3  | `find_container_of_a` | addresses of **live** `struct test` objects (stack, heap, `Vec`-backed arrays, over/under-aligned offsets); result dereferenced to read `.a` back | [x] |
| 4  | `find_container_of_b` | 4096 uniformly random 64-bit pointer values (exercises the `-4` wrap) | [x] |
| 5  | `find_container_of_b` | the same boundary addresses as row 2 (`0…3` wrap below zero) | [x] |
| 6  | `find_container_of_b` | addresses of the `.b` member of **live** objects; result dereferenced to read `.b` back | [x] |
| 7  | `find_container_of_a` + `find_container_of_b` composed | the invariant `main` depends on: `_a(&t.a) == _b(&t.b) == &t` for many live objects at many alignments, with randomized field values read back through both recovered pointers | [x] |
| 8  | `main` | `argc == 3`, both args plain decimal inside `int` range, randomized incl. negatives and zero | [x] |
| 9  | `main` | `argc == 3`, randomized leading whitespace runs built from all six C-locale `isspace` bytes (`' '`, `\t`, `\n`, `\v`, `\f`, `\r`) before the number | [x] |
| 10 | `main` | `argc == 3`, explicit `+`/`-` sign combined with randomized leading-zero padding (`"+000012"`, `"-0"`, `"0000"`) | [x] |
| 11 | `main` | `argc == 3`, digits followed by randomized trailing garbage (`"12abc"`, `"3.9"`, `"7 8"`, `"5-"`) | [x] |
| 12 | `main` | `argc == 3`, subject sequences with **no** digits: random letters/punctuation, lone signs, `"0x"` prefixes, high-bit bytes `\x80…\xff` | [x] |
| 13 | `main` | `argc == 3`, empty-string arguments (one, then both) | [x] |
| 14 | `main` | `argc == 3`, full cross-product of `int` boundary values `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` — exercises sum wrap and the `INT_MIN` `printf` case | [x] |
| 15 | `main` | `argc == 3`, cross-product of `long`/`int` boundary *strings* (`2147483647/8`, `-2147483648/9`, `4294967296`, `LONG_MAX`, `LONG_MAX+1`, `LONG_MIN`, `LONG_MIN-1`) — truncation × saturation | [x] |
| 16 | `main` | `argc == 3`, randomized 19–200-digit numbers, both signs, with and without leading zeros (`strtol` saturation) | [x] |
| 17 | `main` | `argc == 3`, unstructured fuzz: 0–24 uniformly random non-NUL bytes per argument | [x] |
| 18 | `main` | `argc` in `4…9` with matching longer `argv` — extra arguments must be ignored | [x] |
| 19 | `main` | `argv` of ≥ 3 valid entries but a *lying* `argc` (`0`, `1`, `2`, `-1`, `INT_MIN`, `INT_MAX`) — `argc` is never read | [x] |
| 20 | `main` | repeated invocation: the same loaded `.so` called 256 times in one process (no residual state, no buffering drift) | [x] |
| 21 | `find_container_of_a`, `find_container_of_b` | the C reference rebuilt at `-O2` (`C_SO_PATH_O2`) versus Rust, random + boundary pointer corpus | [x] |
| 22 | `main` | the C reference rebuilt at `-O2` versus Rust, over the whole randomized argument corpus of rows 8–19 | [x] |
| 23 | `main` | stdout redirected to a **regular file** (fully buffered) and to a **pipe** — captured bytes compared for both | [x] |
| 24 | whole program | end-to-end: CMake-built C `driver` executable vs the Rust `driver` executable as subprocesses; stdout bytes, exit code *and* terminating signal compared over the randomized corpus | [x] |
| 25 | whole program | stdout is a pipe with **no reader**: the default `SIGPIPE` disposition a C program inherits must be reproduced (Rust's runtime otherwise sets `SIG_IGN`, which would turn a fatal signal into exit status 0) | [x] |
| 26 | `main` | argument lengths of 1 000 / 10 000 / 100 000 bytes: all-digit runs (saturating), long leading-zero runs, long whitespace runs, long digit-free runs | [x] |
| 27 | `main` | input immutability: neither implementation may write to the `argv` strings or the pointer array (checked by snapshotting before/after) | [x] |
| 28 | `main` | `printf("%d\n")` rendering at every digit-count boundary (`±(10ⁿ−1)`, `±10ⁿ`, `±(10ⁿ+1)` for n = 0…9, plus `INT_MIN`/`INT_MAX`) and a random sample — the decimal formatter is the only C behaviour reimplemented rather than transliterated | [x] |

## Row → test mapping

| rows | test file | test names |
|------|-----------|------------|
| 1–23, 26–28 | `tests/differential.rs` | `row01_…` … `row28_…` |
| 24–25 | `tests/end_to_end.rs` | `row24_end_to_end_program_equivalence`, `row24_exit_status_zero_on_success`, `row25_sigpipe_disposition_matches` |

## Running it

```sh
# Every configuration, plus the release profile:
./check_all_features.sh

# Or by hand — note that `cargo test` alone does not build the cdylib, because no
# test target depends on it, so build first to make the tests load the real
# target/<profile>/libdriver.so artifact:
cargo build && cargo test

# Anti-vacuity check: deliberately break one behaviour at a time and confirm the
# suite notices (restores the sources and rebuilds afterwards):
./mutation_check.sh
```

`.cargo/config.toml` sets `RUST_TEST_THREADS = "1"`: the tests must redirect file
descriptor 1 to capture what `printf` writes, and fd 1 is process-wide, so
libtest's own progress output would otherwise land inside a capture window.

## Feature-combination matrix (Phase D)

| combination | `cargo check` | `cargo test` |
|-------------|---------------|--------------|
| *(default — the only one)* | [x] | [x] |
| `--no-default-features` (identical: no features declared) | [x] | [x] |
| `--all-features` (identical: no features declared) | [x] | [x] |
