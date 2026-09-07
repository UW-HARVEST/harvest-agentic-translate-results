# Verification report

`bash translation/run_all.sh` → **ALL CHECKS PASSED**

Everything below is driven through `libloading`: both the C `.so`
(`c_src/build/libharvest-work-YYxvDp.so`) and the Rust cdylib
(`translation/target/{release,debug}/libmaxnmin_lib.so`) are `dlopen`ed and
called through their exported symbols. No Rust function is ever called directly,
so the `#[unsafe(no_mangle)] extern "C"` wrappers are themselves under test.

Because neither library exports its `static` node table and there is no public
reset entry point, the harness gets pristine state per test by copying each
`.so` to a unique temp path and `dlopen`ing the copy (`Pair::fresh()` in
`tests/common/mod.rs`). Each test therefore owns private `node_storage` /
`node_count` in both libraries, and tests are safe to run in parallel.

## Completion gate

| gate | status |
|---|---|
| `SYMBOLS.md`: `nm -D` shows 0 missing / 0 unresolved non-libc symbols in Rust | **PASS** — symbol diff is empty (7/7), both profiles |
| Phase B: every row of `CONFIGS.md` (32 rows) passes across randomized inputs | **PASS** — `tests/differential.rs`, 32/32 |
| Project builds a binary? | **N/A** — `Cargo.toml` has no `[[bin]]`, no `src/main.rs`; `CMakeLists.txt` builds only `add_library(... SHARED)`. No driver stdout to compare. |
| Phase C: every row of `ERRORS.md` (30 rows) has a passing error-path test | **PASS** — `tests/errors.rs`, 25 tests covering all 30 rows |
| Holds under every feature combination | **PASS** — the crate declares no `[features]`, so default is the only combination; verified for both `release` and `debug` profiles |

Counts (all with fixed PRNG seeds, `splitmix64`): ~170 k randomized
`safe_double_to_int` calls, ~70 k randomized `maxnmin` calls (incl. the 9^4
boundary cross-product), ~10 k randomized `process_string` calls, ~2 k
randomized node tables, and 300 randomized 60-step interleaved sequences over
all seven exports.

## Divergence found and fixed

**Rust `.so` aborted (SIGABRT) where C segfaults (SIGSEGV)** on the two NULL
dereference paths — `add_node(.., NULL, ..)` (ERRORS.md row 5) and
`process_string(NULL)` (row 13).

Cause: the `dev` profile enables `debug-assertions`, which makes rustc's
`CheckNull` MIR pass turn `*ptr` on a null pointer into a Rust panic. Panicking
out of an `extern "C"` function aborts, so the child process died with signal 6
instead of the C library's signal 11. The C library is compiled with no runtime
UB instrumentation, so the translation must not have any either.

Fix (`translation/Cargo.toml`):

```toml
[profile.dev]
panic = "abort"
debug-assertions = false
overflow-checks = false
```

After the fix both libraries die with SIGSEGV on both NULL paths, and with the
same signal on the unbounded-recursion path (row 12, self-parent node), in both
profiles.

No divergence was found in any return value of any of the seven functions.

## Harness adequacy (mutation check)

To prove the suite is not vacuous, 15 mutations were injected into
`translation/src/lib.rs`, each built and run through the full suite, then
reverted:

| mutation | result |
|---|---|
| `get_children_count` counts by 2 | CAUGHT (20 tests) |
| `calculate_subtree_sum` seeds `value + 0.5` | CAUGHT (19 tests) |
| `process_string` treats bytes as unsigned | CAUGHT (6) |
| `maxnmin` uses `% 7` for `node_id` | CAUGHT (6) |
| `add_node` capacity check `>` instead of `>=` | CAUGHT (2) |
| `maxnmin` multiplies children by 11 | CAUGHT (10) |
| `maxnmin` parent probe `+ 2` | CAUGHT (9) |
| `safe_double_to_int` returns 1 for NaN | CAUGHT (6) |
| `find_node_by_id` returns `n.add(1)` | CAUGHT (23) |
| `calculate_subtree_sum` iterates children in reverse (f64 order) | CAUGHT (5) |
| `add_node` returns `count` instead of `count - 1` | CAUGHT (19) |
| name truncated at 48 instead of 49 bytes | CAUGHT (5) |
| `strncpy` bound `MAX_NAME_LEN` instead of `MAX_NAME_LEN - 1` | survived — **equivalent**: `name[49] = 0` overwrites the extra byte, so the stored struct is unchanged |
| drop the `&& active` guard in `find_node_by_id` | survived — **equivalent**: `add_node` always stores `active = 1` and indices `>= node_count` are never scanned (this is exactly ERRORS.md row 7, documented as unreachable via the public API) |
| `safe_double_to_int` clamp `>=` instead of `>` | survived — **equivalent**: at `d == (double)INT_MAX` both the clamp and `(int)d` yield `INT_MAX` |
| `process_string` drops the redundant `if (*str)` guard | survived — **equivalent**: the `while` loop already tests the same condition |

All four survivors are provably semantically equivalent to the C, so the
mutation score is effectively 100 %.

## Reproduce

```sh
bash translation/run_all.sh          # everything: C build, symbol diff, all profiles
# or, individually:
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release && cargo test
```
