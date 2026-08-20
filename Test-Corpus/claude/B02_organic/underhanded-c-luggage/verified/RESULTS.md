# RESULTS.md — verification of the C → Rust translation of `luggage.c`

Ground truth: `c_src/src/luggage.c` (never modified).  Everything below was
produced by running the C artifacts and the Rust artifacts side by side and
comparing their bytes.

## Artifacts

| artifact | how it is built |
|---|---|
| C executable | `cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .` → `c_src/build/driver` |
| C shared library | `gcc -shared -fPIC -O0 -Dmain=luggage_main -o cbuild/libluggage.so c_src/src/luggage.c` (compiler flags only — `c_src/` is untouched; `main` is renamed so the translation unit can be dlopen'd) |
| Rust executable | `cargo build [--release]` → `target/<profile>/driver` |
| Rust shared library | same build → `target/<profile>/libdriver.so` (`crate-type = ["cdylib", "rlib"]`) |

`src/luggage_core.rs` holds the whole translation and is shared by the binary
(`src/main.rs`) and the C-ABI layer (`src/lib.rs`), so the `.so` tests and the
executable tests exercise the *same* code.

Run everything with:

```
./check_all.sh
```

Last run:

```
=== 1. feature combinations declared in Cargo.toml ===
  no [features] table -> exactly one configuration (the empty/default one)
  1 combination(s): []
=== 2. cargo check for every combination ===
  [ok]   cargo check --no-default-features
  [ok]   cargo check <default features>
  [ok]   cargo check --all-features
=== 3. build the C and the Rust artifacts ===
  [ok]   C executable  c_src/build/driver
  [ok]   C shared lib  cbuild/libluggage.so
  [ok]   Rust debug    target/debug/{driver,libdriver.so}
  [ok]   Rust release  target/release/{driver,libdriver.so}
=== 4. symbol parity (nm -D --defined-only) ===
  C .so exports:    addRoutingDirectiveToList luggage_main matches printMatchingDirectives superseded supersedes
  Rust .so exports: addRoutingDirectiveToList luggage_main matches printMatchingDirectives superseded supersedes
  [ok]   0 symbols missing from the Rust .so
=== 5. differential test suite for every combination ===
  [ok]   cargo test --no-default-features   (66 tests)
  [ok]   cargo test <default features>      (66 tests)
  [ok]   cargo test --all-features          (66 tests)
  [ok]   cargo test --release               (66 tests)
=== summary ===
  ALL CHECKS PASSED
```

## Test suite layout

| file | what it does | tests |
|---|---|---|
| `tests/differential_exec.rs` | runs the C and the Rust **executable** as subprocesses with identical argv + stdin and compares stdout, stderr and exit status byte-for-byte (Phase B rows `p01`–`p33`, Phase C rows `e01`–`e24`) | 57 |
| `tests/differential_ffi.rs` | `libloading`-loads **both `.so`s** and calls only their exported C symbols: `matches`, `supersedes`, `superseded`, `addRoutingDirectiveToList`; plus `nm -D` symbol parity and the `repr(C)` layout check | 8 |
| `tests/differential_ffi_print.rs` | `printMatchingDirectives` through both `.so`s, capturing file descriptor 1 (own test binary so no other test can write to fd 1 while it is redirected).  Contains `f02_print_null_list` + `f15_print_random`, driven by one `#[test]`. | 1 |
| `tests/support/mod.rs` | artifact building/locating, subprocess comparison, seeded splitmix64 PRNG, input generators | — |
| `tests/support/ffi.rs` | C-layout `Node`, exported-function signatures, `dlopen` helpers, node generators, fd-1 capture | — |
| `fuzz_diff.py`, `edge_diff.py` | stand-alone Python differential fuzzers used while investigating (≈ 8 000 random cases + 103 hand-written cases, 0 mismatches) | — |

Randomization is property-style with **fixed seeds** (`Rng::new(0x…)` per test),
so every run covers the same thousands of inputs reproducibly.  Rough case
counts: ~4 500 randomized subprocess comparisons (Phase B), ~600 error-path
comparisons (Phase C), 4 000 `matches` + 1 500 `supersedes` + 1 500 `superseded`
+ 2 000 `addRoutingDirectiveToList` single-insert + 300 full list build-ups +
400 `printMatchingDirectives` comparisons through the `.so` boundary.

## Test-suite sensitivity (mutation checks)

To prove the suite is not vacuous, deliberate bugs were injected into
`src/luggage_core.rs` and the suite was re-run (each mutation was reverted
afterwards; `diff` confirmed the restore):

| mutation | caught by |
|---|---|
| `>` → `>=` in `add_routing_directive_to_list` (tie order) | `p03`, `p04`, `p05`, `p06`, … and `f13`, `f14` (`.so`) |
| `{:010}` → `{:09}` in the `printf` emulation | 8+ exec tests |
| `supersedes` keeps searching after the first luggage-id match | `e20`, `p07`, `p15`, `p17`, `p19`, `p26` and `f11`, `f12` (`.so`) |
| `matches` loses the `-` wildcard | `f10` (`.so`) + exec filter tests |
| `c_str` stops truncating at the NUL byte | `e23`, `p02`, `p05`, `p06`, `p07`, … |
| removing the `SIGPIPE` restore in `src/main.rs` | `p32` (`C=Some(141)` vs `Rust=Some(0)`) |

## Divergences found and fixed during verification

1. **`SIGPIPE` disposition** — a C program starts with `SIGPIPE` at `SIG_DFL`,
   whereas the Rust runtime sets it to `SIG_IGN`.  `./driver - - - - | head -c 20`
   therefore ended with status **141** for C but **0** for Rust.
   Fixed in `src/main.rs` (`restore_default_sigpipe()`), covered by `p32`.
2. **Performance parity of the O(n²) supersede walk** — `c_str`/`c_str_eq`
   allocated a `Vec` per comparison, which made a 100 000-record input take
   >120 s in Rust versus ~50 s in C.  `c_str` now returns a slice and
   `superseded` no longer clones its fields; the same input now takes ~76 s
   (behaviour unchanged, verified byte-identical output).

## Known, documented deviation (UB only)

`addRoutingDirectiveToList(NULL, …)`, `superseded(NULL)` and `matches(NULL, …)`
dereference their arguments unconditionally in C; called through the `.so` they
kill the process with `SIGSEGV` (status 139, verified with `ctypes`).  The Rust
exports return instead of reproducing the crash.  This is C undefined behaviour
and is unreachable from the program itself (`main` always passes
`&directive_list_head`, and `argv[i]` is never NULL), so no defined behaviour
differs.  The *defined* NULL cases — `supersedes(NULL, …)` → `0` (ERRORS.md row
32) and `printMatchingDirectives(NULL, …)` → no output (row 34) — are asserted
equal in `f01`/`f02`.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D --defined-only` on the C `.so` lists exactly six
      code symbols (`addRoutingDirectiveToList`, `supersedes`, `superseded`,
      `matches`, `printMatchingDirectives`, `luggage_main`); the Rust `.so`
      exports all six with identical names.  Symbol diff (C minus Rust) is
      **empty**, checked mechanically by `check_all.sh` step 4 and by
      `symbol_parity_c_so_vs_rust_so`.  No undefined non-libc symbols in the Rust
      artifacts.  Every C function and the single C source file are translated —
      nothing stubbed, no `unimplemented!()`.
- [x] **Phase B** — all **42** rows of `CONFIGS.md` pass, each across many
      randomized inputs with fixed seeds; the lowest-level entry points are
      driven directly through the `.so` boundary (rows 30–36) in addition to the
      end-to-end process interface.
- [x] **Phase C** — all **44** rows of `ERRORS.md` have a passing differential
      test asserting the *same* sentinel (exit code 1 + the exact stderr string,
      `EOF`-driven record dropping, `0`/`1` returns, `SIGPIPE`), plus the generic
      boundaries: NULL pointers, empty/zero-length and oversized inputs, values
      one step past every documented range (`INT_MAX±1`, `UINT_MAX±1`,
      `LONG_MAX±1`, `LONG_MIN±1`, widths 8/6/3/3/80 ±1).  Out-of-range enum
      values do not apply — the C source declares no enums (row 44).
- [x] **All feature combinations** — `Cargo.toml` declares no `[features]`, so
      the single valid combination is the empty one; it is verified as
      `--no-default-features`, with default features and with `--all-features`,
      in both the debug and the release profile (66 tests each, all passing).
