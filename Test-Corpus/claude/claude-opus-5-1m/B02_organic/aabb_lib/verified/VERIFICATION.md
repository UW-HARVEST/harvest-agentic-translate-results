# VERIFICATION.md — completion gate

Differential verification of `src/lib.rs` (Rust) against `c_src/src/lib.c` (C,
the ground truth) for this cute_c2-style 2D collision library.

Reproduce everything with:

```sh
./run_verification.sh     # phases A-D, every feature combination
./mutation_check.sh       # test-strength evidence
```

## Artifacts

| file | phase | content |
|------|-------|---------|
| `SYMBOLS.md` | A / D | every symbol from `nm -D` on the C `.so`, matched against the Rust `.so` |
| `ERRORS.md` | A / C | the error-surface table: 71 rows, one per distinct rejection/guard in the C |
| `CONFIGS.md` | A / B | the configuration-surface table: 63 rows of option × input-shape combinations |
| `MUTATION_NOTES.md` | — | proof the suite actually observes the code (28/31 mutants killed; 3 proven equivalent) |
| `tests/common/mod.rs` | — | harness: loads both `.so`s via `libloading`, bit-exact comparison, seeded RNG |
| `tests/phase_b_valid.rs` | B | 56 valid-path differential tests |
| `tests/phase_c_errors.rs` | C | 51 error-path differential tests (+5 `#[ignore]`d UB rows) |
| `tests/smoke.rs` | — | quick leaf-function probe |
| `check_symbols.sh` | D | symbol-parity gate (exits non-zero if the diff is non-empty) |
| `run_verification.sh` | A-D | driver: builds the C `.so`, enumerates feature combos, checks/builds/tests each |
| `mutation_check.sh` | — | mutation testing |

Nothing under `c_src/` was modified. The only change to `Cargo.toml` is
`libloading = "0.8"` under `[dev-dependencies]`.

## Build configurations (Phase A step 1)

`Cargo.toml` has **no `[features]` table**, and the C build has no compile-time
axes either:

```
$ grep -c '^\[features\]'  Cargo.toml                          -> 0
$ grep -cE '^[[:space:]]*#[[:space:]]*(if|ifdef|ifndef)' c_src/src/lib.c  -> 0
$ grep -cE 'option\(|add_definitions|target_compile_definitions' c_src/CMakeLists.txt -> 0
```

So the power set of features has exactly **one** element. `run_verification.sh`
enumerates it mechanically and additionally runs the `default` and
`--all-features` invocations (identical here, checked anyway):

| # | invocation | check | build | symbols | tests |
|---|------------|-------|-------|---------|-------|
| 1 | `cargo … --no-default-features` | ok | ok | 38/38 | 108 pass |
| 2 | `cargo …` (default) | ok | ok | 38/38 | 108 pass |
| 3 | `cargo … --all-features` | ok | ok | 38/38 | 108 pass |

## How the tests call the code

Both libraries are loaded with `libloading` and driven **only** through their
exported C symbols — the Rust side is never called as a Rust crate, so the
`#[unsafe(no_mangle)] extern "C"` wrappers, the `repr(C)` struct layouts and the
SysV return-value classification (`c2v` in `xmm0`, `c2x` in `xmm0`/`xmm1`,
`c2Capsule` on the stack) are all part of what is being verified.

Comparisons are **bit-exact**: every float in every output is compared via
`f32::to_bits`, so `+0.0` vs `-0.0` and NaN sign/payload differences fail the
test. Out-parameters are pre-filled with poison values so "not written" is
distinguishable from "written".

## Completion gate

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing symbols in the Rust `.so`.**
      C exports 38, Rust exports 38, `comm -23` and `comm -13` are both empty.
      No stubs were used: `c_src` has one translation unit and all 38 of its
      external-linkage functions have real Rust bodies. `nm -D --undefined-only`
      on the Rust `.so` lists only libc/libgcc runtime imports — 0 undefined
      non-libc symbols.
- [x] **Phase B: every row in `CONFIGS.md` passes across randomized inputs.**
      63/63 rows, 56 test functions, fixed-seed property-style inputs. Includes
      the lowest-level entry points (`c22`, `c23`, `c2D`, `c2L`, `c2Witness`,
      `c2Support`, `c2MakeProxy`, `c2GJK` with all cache/transform/out-param
      combinations), not just the convenience wrappers. `aabb()` reaches all 8
      of its possible return values.
- [x] **Phase C: every row in `ERRORS.md` has a passing error-path test.**
      71/71 rows. 66 executed differentially; 5 (rows 02, 33, 34, 50, 68) are
      compiled-but-`#[ignore]`d because the C reference itself is undefined
      there (NULL dereference, out-of-bounds stack write, read of uninitialised
      stack memory), so there is no input-determined C result to diff.
      Generic boundaries are covered too: all 64 nullable-pointer combinations,
      zero/negative/oversized counts, one-past-range `count` values, and
      out-of-range `C2_TYPE` enum values (`3`, `4`, `5`, `7`, `100`, `INT_MAX`,
      `0x80000000`, `(unsigned)-1`) through `c2Collided` and `c2MakeProxy`.
- [x] **All of the above hold under every feature combination** — the single
      combination, plus `default` and `--all-features`, all verified by
      `run_verification.sh`.

## Notable findings

1. **No behavioural divergence was found.** The translation is already
   bit-exact, including NaN payload/sign propagation, signed zeros, overflow to
   `±inf`, denormal underflow, and the library's own quirks (exact tangency
   counts as *not* colliding; a `NaN` AABB collides with everything; the GJK
   cache metric test at `lib.c:404` is dead code that always accepts the cache).
   No change to `src/lib.rs` was required.

2. **`cargo test` alone can silently test a stale library.** The crate is
   `crate-type = ["cdylib"]` and the tests only `dlopen` it, so cargo has no
   dependency edge from any test target to the library and never rebuilds it for
   `cargo test`; and because cargo's fingerprinting is mtime-based, restoring a
   file with `mv`/`git checkout` can even move the mtime backwards and make a
   stale artifact look current. This produced a real false pass while building
   `mutation_check.sh`. Fixed by `rebuild_cdylib()` +`assert_fresh()` in
   `tests/common/mod.rs` and an explicit `cargo build` in the driver script.
   See `MUTATION_NOTES.md`.

3. **The C's only reachable-looking uninitialised read is unreachable.**
   `c2GJK` declares `c2Simplex s;` uninitialised, and if the loop ever left via
   the `iter < 20` condition it would read the never-written `u` of the vertex it
   appended last. Measured maximum iteration count over 108 000 adversarial
   queries: **5** (`e35_gjk_iteration_cap`). So that path is dead and the
   Rust's zero-initialised simplex can never be observably different.

4. **Three of the four documented UB rows are reachable only through the cache
   API** (`cache->count > 3`, or cached indices `>= proxy->count`), and the
   fourth (`c2GJK` with an out-of-range `C2_TYPE`) reads the uninitialised
   `c2Proxy`. `CONFIGS.md` row 46 is deliberately restricted to type switches
   where the cached indices stay in range, so the warm-start path is still
   exercised end to end without invoking UB.
