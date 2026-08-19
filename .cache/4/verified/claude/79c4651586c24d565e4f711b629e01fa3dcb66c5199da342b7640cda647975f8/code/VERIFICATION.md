# VERIFICATION.md — completion gate

Reproduce everything with:

```sh
./run_all.sh
```

## Phase A — surface map

| artifact | content |
|----------|---------|
| `SYMBOLS.md` | every `nm -D` symbol of the C `.so`; 1 public export (`driver`), present in the Rust `.so` |
| `ERRORS.md` | 13 rows — every rejection branch grepped out of `c_src/src/driver.c` |
| `CONFIGS.md` | 16 rows — the full cross-product of the three branch axes plus state/multiplicity/magnitude axes |

Feature combinations enumerated from `Cargo.toml` (`[features] default = []`,
no other feature) and `c_src/CMakeLists.txt` (no `option()`, no
`target_compile_definitions`, no `#ifdef` in the sources):

| # | invocation | `cargo check` |
|---|------------|---------------|
| 1 | `--no-default-features` | ok |
| 2 | (default) | ok |
| 3 | `--all-features` | ok |

## Phase B / C — differential tests

`tests/differential.rs` loads **both** shared libraries with `libloading` and
calls only their exported `driver` symbol; nothing is called directly in-process,
so the `#[unsafe(no_mangle)] extern "C"` wrapper is under test too. Because
`driver` returns `void`, each row captures the process-wide `stdout` file
descriptor around the FFI call and compares the two byte streams exactly.

29 rows (16 from `CONFIGS.md`, 13 from `ERRORS.md`) pass under all three feature
combinations, in both `debug` and `release` (`panic = "abort"`).

## Two harness pitfalls found and fixed

1. **`cargo test` does not build a cdylib-only lib target.** An integration test
   can only link against `lib`/`rlib`/`dylib`, so Cargo never produced
   `libdriver.so` for `cargo test`, and the rows silently compared against a
   `.so` left over from an earlier `cargo build`. Proven by mutating
   `src/lib.rs` and watching all 29 rows still "pass". `tests/differential.rs`
   now runs `cargo build --lib` itself and additionally refuses to start if the
   `.so` is older than anything in `src/` (`assert_rust_so_is_fresh`).
2. **libtest's parallel runner writes to fd 1 while a capture is active**, which
   would drop `test … ok` lines into a captured stream. The test target is
   declared `harness = false` so the rows run sequentially from a custom `main`
   and nothing else touches fd 1.

Each row also asserts the C capture is non-empty, and the fixed transcripts
(`compare_expect`) pin the exact bytes derived from `driver.c` — so a broken
capture cannot make a row pass by comparing `""` with `""`.

## Mutation validation of the harness

Each mutation was applied to `src/lib.rs`, the suite run, then reverted:

| mutation to the Rust translation | rows failed |
|----------------------------------|-------------|
| `if x != 1` → `if x != 2` | 21 |
| `if Y.load(..) != 2` → `!= 3` | 15 |
| `if z != 3` → `if z != 4` | 11 |
| `"Error: x != 1\n"` → extra space | 17 |
| drop `"Operation failed\n"` from the x path | 17 |
| `"Result: %d\n"` → `"Result: %d \n"` | 29 |
| remove the `Y.store(local_y, …)` assignment | 14 |
| x-path `result = 1` → `4` | 17 |
| y-path `result = 2` → `0` | 13 |
| z-path `result = 3` → `0` | 9 |
| success `result = 0` → `1` | 9 |
| skip the `x != 1` check (`if false`) | 17 |

Every behaviour-changing mutation is caught. The one mutation that is *not*
caught — changing the initializer `static int y = 123` to any other value — is
genuinely unobservable through the public ABI, because `driver` assigns
`y = local_y` before the first read of `y`; there is no code path that can
observe the initializer.

## Gate

- [x] `SYMBOLS.md`: `nm -D` diff empty; 0 undefined non-libc symbols in the Rust `.so`.
- [x] Phase B: all 16 `CONFIGS.md` rows pass across randomized (fixed-seed) inputs.
- [x] Phase C: all 13 `ERRORS.md` rows have a passing error-path differential test.
- [x] All of the above hold under every feature combination (`--no-default-features`,
      default, `--all-features`) and in both debug and release.
