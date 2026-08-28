# Verification report

C ground truth: `c_src/src/lib.c` (199 lines) + `c_src/include/lib.h`.
Rust translation: `translation/src/lib.rs`.

Every test loads **both** shared objects with `libloading` and calls **only**
exported C symbols — the Rust functions are never invoked directly, so the
`#[no_mangle] extern "C"` wrappers are part of what is under test.

## Result

**No behavioural divergence was found.** `translation/src/lib.rs` required no
changes. The two fixes made during verification were both to my own test code
(an over-strict `assert!(got > 0)` where `mode=1, seed=0` legitimately sums to
`0`, and moving the alternate C builds out of a volatile `$TMPDIR`).

## What was compared

Both the **return value** and the **stdout log bytes**. The C lowers
`LOG_MSG(...)` → `printf("[LVL] msg\n")` → `puts("[LVL] msg")`; LLVM performs
the same lowering for the Rust side (confirmed: neither `.so` imports `printf`,
both import `puts`). The harness taps file descriptor 1 around each call and
compares the emitted bytes exactly, so the `[INFO]` / `[WARNING]` / `[ERROR]`
lines and their ordering are verified, not just the integer result.

## Test inventory (79 tests)

| file | tests | scope |
|------|-------|-------|
| `tests/smoke.rs`              | 4  | harness self-check: both `.so`s load, all 4 symbols resolve, tap really captures |
| `tests/phase_b_configs.rs`    | 39 | one test per `CONFIGS.md` row, randomized (fixed seed) |
| `tests/phase_b_exhaustive.rs` | 8  | whole axes enumerated, not sampled (all 65 536 seeds, all thresholds 0..=3000, all `iterations` 0..=1024 and 65 400..=65 535, both validity edges, all reachable op inputs) |
| `tests/phase_c_errors.rs`     | 19 | one test per `ERRORS.md` row (26 rows, some grouped by shared trigger) |
| `tests/phase_d_symbols.rs`    | 5  | `nm -D` parity, expected/internal symbol sets, no undefined non-libc symbols, `dlsym` resolution, Phase A artifacts present |
| `tests/robustness.rs`         | 4  | reentrancy from 8 threads; RSS-growth leak checks on the success and error paths |

Roughly 10 million differential calls per suite run.

## Configuration matrix — 42 runs, all green

| axis | values |
|------|--------|
| C build | CMake default, `gcc -O0/-O2/-O3/-Os`, `clang -O0/-O2` (7) |
| Rust `.so` | `target/release` (`panic = "abort"`), `target/debug` (overflow checks on) (2) |
| feature combo | default, `--no-default-features`, `--all-features` (3) |

Multiple C optimisation levels matter here because the C source contains signed
`+`/`*` on `int` (formally UB) and reads a `malloc(0)` result; the Rust must
agree with every lowering, not just one. It does.

`Cargo.toml` declares **no `[features]` table**, so the three combos are the
same build — proved rather than assumed: the compiled test binary is
byte-identical across all three
(`sha256 = 5c123737c5e6f7a6706b6ed8c0ca284cebddc16e7bb1abb8d59e602d0778e52c`).

Reproduce with `./run_all.sh tier1`, `tier2a`, `tier2b`, `tier3a`, `tier3b`
(split because the whole matrix exceeds a 600 s budget).

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` C→Rust symbol diff is **empty**
      (`double_value`, `gotomach`, `process_value`, `triple_value`); 0 undefined
      non-libc symbols in the Rust `.so`; no stubs, no `unimplemented!()`; the
      four `static` helpers and three macros correctly produce no symbols on
      either side. Enforced by `d1`–`d3`.
- [x] **Phase B** — all 39 `CONFIGS.md` rows pass across randomized inputs,
      plus 8 exhaustive whole-axis sweeps.
- [x] **Phase C** — all 26 `ERRORS.md` rows have a passing differential test
      asserting the *same sentinel* (`-1` / `-2`, and that `-3`/`-4`/`-5`/`-6`
      are never produced), including out-of-range `mode` "enum" values, null and
      bogus `void*` context pointers, zero/oversized lengths, and one-step-past
      boundaries on every axis.
- [x] **Every feature combination** under every C build and both Rust profiles.

## Notes on the C's unreachable error paths

`ERRORS.md` rows 6–12 (`-3`, `-4`) and 9–10 (`-5`, `-6`) cannot be triggered
through the FFI surface: the API takes only `int`s, so the largest allocation is
`65535 * 4` bytes; `status` is unconditionally `1`; and `count <= i < iterations
== capacity` makes `is_valid_state` always true. Rather than skip them, the
tests assert the *reachable* half of each contract — that both implementations
**never** return those sentinels for any input (including a 50 000-case
full-`i32` sweep and an exhaustive `iterations` 0..=512 append-everything
sweep), and that both ends of the allocation range (`malloc(0)` and the 256 KiB
maximum) are treated as success by both.

## Files added

```
translation/SYMBOLS.md          Phase A: symbol surface
translation/ERRORS.md           Phase A: error-surface table (26 rows)
translation/CONFIGS.md          Phase A: configuration-surface table (39 rows)
translation/VERIFICATION.md     this report
translation/run_all.sh          full matrix runner
translation/.cargo/config.toml  RUST_TEST_THREADS=1 (the fd-1 tap needs serial tests)
translation/tests/              common/mod.rs + 6 test files
```

`Cargo.toml` gained only `[dev-dependencies] libloading = "0.8"`.
Nothing under `c_src/` was modified — only `c_src/build/` was created, as the
task instructed (source checksums and 01:17 mtimes unchanged).
