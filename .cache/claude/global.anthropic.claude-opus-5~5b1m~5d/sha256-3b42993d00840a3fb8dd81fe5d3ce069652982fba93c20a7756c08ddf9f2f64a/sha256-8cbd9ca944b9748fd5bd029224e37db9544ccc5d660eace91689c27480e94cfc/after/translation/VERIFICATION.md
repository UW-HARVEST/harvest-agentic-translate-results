# Verification report

C ground truth: `c_src/src/lib.c` (187 lines, one translation unit, 5 exported
functions). Rust under test: `translation/src/lib.rs` → `libenvy_lib.so`
(`crate-type = ["cdylib"]`).

Every comparison is made **through the `.so` boundary**: both libraries are
`dlopen`ed with `libloading` and every function is invoked via its exported C
symbol, so the `#[no_mangle] extern "C"` wrappers are themselves under test.
No Rust function is ever called directly.

Reproduce everything with:

```sh
cd translation
bash run_all.sh          # all four phases, all feature combos, all cross-checks
bash mutation_check.sh   # proves the suite detects divergence
```

## Completion gate

| gate | status | evidence |
|------|--------|----------|
| `SYMBOLS.md`: `nm -D` shows 0 missing / 0 unresolved non-libc symbols in Rust | **PASS** | `tests/symbols.rs` (3 tests); `run_all.sh` step 4 prints `symbol diff: EMPTY` |
| Phase B: every row of `CONFIGS.md` (C1–C50) passes across randomized inputs | **PASS** | `tests/configs.rs` — 50 tests, 50 passed |
| Binary/driver stdout compared byte-for-byte | **N/A, asserted** | No executable is built (`add_library(... SHARED ...)` only; `crate-type = ["cdylib"]` only). `c50_no_driver_binary_is_built` fails if that ever changes. stdout is instead compared per-call via fd redirection. |
| Phase C: every row of `ERRORS.md` (E1–E36) has a passing error-path test | **PASS** | `tests/errors.rs` — 36 tests, 36 passed |
| All of the above under EVERY feature combination | **PASS** | `Cargo.toml` declares no `[features]`, so the combination set is `{default}`. `run_all.sh` additionally runs `--no-default-features`: both PASS. |

Test totals: **92 tests, 92 passed, 0 failed** (`configs` 50, `errors` 36,
`symbols` 3, `smoke` 3).

## How outputs are compared

For every call, the harness compares **three** things, not just the return value:

1. the returned `int`;
2. every byte written to **stdout** (`printf`);
3. every byte written to **stderr** (`fprintf(stderr, ...)`).

fd 1 and fd 2 are `dup2`'d onto scratch files around each call and `fflush(NULL)`
is issued before reading, so buffering cannot hide or reorder output. Both `.so`s
share the process's single libc — hence the same `FILE *stdout` — and the Rust
translation calls the *same* libc `printf`/`fprintf`/`snprintf`/`atoi`/`getenv`/
`strchr`/`memcpy` rather than reimplementing them, so formatting and `atoi`
edge-case behaviour are identical by construction.

Where a `struct ConfigFlags *` is passed, **all four bytes** of the allocation
unit are compared after the call, which is what catches bit-field layout and
padding-write mistakes.

## Bugs and hazards found and fixed during verification

### 1. Harness was testing a stale `.so` (critical, harness bug)

`cargo test` builds the integration-test binaries but **does not rebuild the
`cdylib`** — nothing links against it. The suite therefore kept `dlopen`ing an
artifact left over from an earlier `cargo build --release`. Proof: a mutation
run against the un-fixed harness reported **0 of 28** deliberate breakages
detected. Every test "passed" while comparing the C library against a Rust
library that no longer matched its own source.

Fixed in `tests/common/mod.rs`: `find_rust_so()` now shells out to
`cargo build --release --offline` and then **asserts the `.so` is newer than
every `src/*.rs`**, panicking with `STALE ARTIFACT` otherwise. The same mtime
guard is applied to the C `.so` versus `c_src/src/lib.c`. After the fix the same
mutation run detects **27 of 28** (the 28th is a provably equivalent mutant, see
below).

### 2. `&mut *ptr` on a caller-supplied pointer diverged from C (real translation fix)

`init_config_from_env`, `perform_operation` and `apply_bit_operations` used
`let flags = &mut *flags;` / `&*flags` to reach the bit-fields. A C caller may
pass **any** pointer — C emits no validity check — so forming a Rust reference
attaches a guarantee the input does not carry. With `debug_assertions` on, the
`debug` profile aborted with **SIGABRT** ("thread caused non-unwinding panic")
where the C library performs the load/store and takes **SIGSEGV**. Caught by
`err_e34_null_flags_pointer_faults_identically` once the debug cdylib was
cross-checked.

Fixed by replacing the reference-forming accessors with raw-pointer
read/write helpers (`cf_get`/`cf_set`/`cf_verbose`/…) that never create a
reference. Both profiles now fault identically, and the release build is
unchanged. Re-verified: release **and** debug cdylibs both pass all 92 tests.

### 3. Harness stdout capture raced with libtest (harness bug)

fd redirection is process-global, so libtest's own progress output ("test foo
... ok") landed inside the captured buffer and 17 tests failed spuriously.
Fixed by pinning `RUST_TEST_THREADS = "1"` in `translation/.cargo/config.toml`,
so `cargo test` is correct however it is invoked, rather than depending on the
caller remembering `-- --test-threads=1`.

## Behaviours of the C that were replicated, not "fixed"

| C behaviour | why it looks wrong | replicated in Rust |
|---|---|---|
| An env var set to `""` is **not** treated as unset: `parse_env_numeric` returns `atoi("") == 0`, **not** `default_val` | looks like a missing empty check | yes — `c3`, `err_e5` |
| `PROG_OPTIMIZE=""` / `=0` / `=false` all **enable** optimize (only NULL-ness is tested) | looks like a bug | yes — `err_e14` |
| `PROG_VERBOSE=true` is **false**, `PROG_VERBOSE=xx1xx` is **true** (`strchr(v,'1')`) | substring test instead of parse | yes — `err_e12`, `err_e13`, `c14` |
| The comma check precedes the semicolon check, so `"1,2;3"` emits **only** "Invalid character" | the semicolon branch is dead for such values | yes — `err_e4` |
| `,` and `;` are the *only* rejected characters; `abc`, `0x1f`, `--3`, overflowing digits all flow into `atoi` unchecked | looks like weak validation | yes — `err_e6`, `err_e7`, `err_e8` |
| `perform_operation` computes `int result = 0;` then unconditionally overwrites it on both branches | dead store | yes (kept for fidelity) |
| `operation_mode = 0755` is computed and only ever *printed* | unused value | yes |
| `snprintf` into `buffer`, then two `strchr` calls whose results only gate debug prints | the buffer is otherwise unused | yes — `err_e30` shows both colons are always present, so the NULL guards are dead code |
| On `result < 0` the code `memcpy`s the backup over `state` and returns `state.base_value` — i.e. discards all arithmetic and returns `param1` | surprising recovery semantics | yes — `err_e24`, `err_e32`, `c43` |
| `state`, `state_backup` and `buffer` are **uninitialised** automatics | reading them would be UB | the Rust zeroes them; not observable, because every field and buffer byte the C later reads is unconditionally written first (all six bit-fields by `init_config_from_env`, the three members by assignment, the buffer by `snprintf`). Only never-read padding differs, and `c12`/`c21`/`c27`/`err_e15` prove caller-visible padding is handled identically. |
| Signed overflow in `val1 * log_level`, `val1 + val2`, `result += …`, and `adjusted << 1` on negative/large values; `param4 >> 2` on negatives | formally UB / implementation-defined in ISO C | matched to gcc's actual behaviour (2's-complement wrapping, arithmetic right shift) via `wrapping_*` and a `u32` round-trip for the left shift. Verified against gcc at **both `-O0` and `-O2`** (`run_all.sh` step 3b) over the full `int` boundary cross-product. |

## Bit-field layout, verified against gcc codegen

`objdump -d` of `init_config_from_env` shows gcc emits **byte-sized**
read-modify-write against **byte 0 only**:

```
movzbl (%rax),%edx ; and $0xffffff8f,%edx ; or $0x30,%edx ; mov %dl,(%rax)
```

Bytes 1–3 of the 4-byte allocation unit are never written. The Rust models
`ConfigFlags` as `#[repr(C, align(4))] { storage: [u8; 4] }` with byte-0-only
RMW, so a caller's `0xFF` in bytes 1–3 survives the call in both libraries —
asserted by `err_e15_dirty_padding_preserved` and by the "bitfield whole-word
store" mutation, which is DETECTED.

## Suite is proven to have teeth (mutation self-check)

`bash mutation_check.sh` injects 28 single-point defects into
`translation/src/lib.rs`, rebuilds, and re-runs the full suite:

**27 / 28 DETECTED** (3–26 failing tests each), covering: `log_level` constant,
`0x0F` mask, `>> 2` shift amount, `<< 1` shift amount, `;` vs `:` in the
rejection check, both octal defaults (`0100`, `012`), `/2` divisor, dropping the
`strchr` in the verbose test, adding a bogus `'1'` test to optimize, `< 0` vs
`<= 0` rollback boundary, whole-word vs byte-0 bit-field store, warning text,
`%o` vs `%d`, rollback return value, `DEBUG_SHIFT`, `log_level` field width,
hand-rolling `atoi`, both `!= 0` guards, `reserved` not cleared,
`cache_enabled` value, double-adding `base_offset`, `+` vs `-` on the optimize
path, removing the comma check, swapping two `printf`s, and sending a warning to
stdout instead of stderr.

The single survivor, `%ld` → `%d` in `"Found colon at position: %ld\n"`, is an
**equivalent mutant, not a gap**: the printed value is always exactly `6`, and on
the x86-64 SysV ABI the argument travels in the same 64-bit register either way,
so `%d` reads its low 32 bits and printf emits the identical byte. No input can
distinguish the two.

## Cross-checks beyond the four phases

| cross-check | result |
|---|---|
| Rust release `.so` vs C at `-O0` (the CMakeLists default) | PASS (92/92) |
| Rust release `.so` vs C at `-O2` (`-DCMAKE_BUILD_TYPE=Release`, built into `translation/target/`, `c_src/` untouched) | PASS (92/92) |
| Rust **debug** `.so` (panic=unwind, `debug_assertions` on) vs C | PASS (92/92) — this is what exposed bug #2 |
| `cargo check` + `cargo test` under `default` and `--no-default-features` | PASS |
| `nm -D` symbol diff | EMPTY |
| Rust `.so` imports resolve via `dlsym(RTLD_DEFAULT)`; none of the 5 API names is imported | PASS |

## Scale of the differential sweep

Randomized with a fixed-seed SplitMix64 PRNG (reproducible), plus exhaustive
sweeps where the space is small enough:

* all **256** `ConfigFlags` byte-0 bit patterns × random values, for
  `perform_operation`, `apply_bit_operations` and `init_config_from_env`
  (this is the out-of-range-enum-across-FFI case: every `int` is a valid C
  bit-field object, and `reserved = 1`, `cache_enabled = 0` and
  `log_level ∈ {0,1,2,4,5,6,7}` are states `init_config_from_env` can never
  produce and are reachable only through the low-level exports);
* the full **9⁴ = 6561** `int`-boundary cross-product of `envy`'s four params;
* the full **3×3×3×5×5 = 675** environment cross-product for `envy`
  (verbose × debug × optimize × base_offset × multiplier), each with randomized
  and fixed param tuples;
* the full **3×3×3 = 27** option cross-product for `init_config_from_env`, each
  against pre-zeroed, `0xFF`-dirty and randomly-dirty flag storage;
* a caller-composed **low-level pipeline** (`init_config_from_env` →
  `parse_env_numeric` ×2 → `perform_operation` → `apply_bit_operations`)
  mirroring `envy`'s internal call order, including flag states `envy` itself
  can never reach — this is the "bugs in the composed pipeline" case that
  per-wrapper tests miss;
* a search for `envy` inputs that land exactly on the `result < 0` rollback
  boundary (`result == -1` rolls back, `result == 15` does not; `result == 0` is
  unreachable with the default config because `apply_bit_operations` ORs `0x0F`
  and `base_offset = 64` is a multiple of 16 — reached instead via
  `PROG_BASE_OFFSET=-15`).

## Files

| file | role |
|---|---|
| `SYMBOLS.md` | Phase A — symbol surface, layout notes, `nm -D` verification command |
| `ERRORS.md` | Phase A/C — error-surface table, E1–E36, all checked |
| `CONFIGS.md` | Phase A/B — configuration-surface table, C1–C50, all checked |
| `VERIFICATION.md` | this report |
| `run_all.sh` | builds everything, runs all phases × all feature combos × cross-checks |
| `mutation_check.sh` | proves the suite detects divergence |
| `tests/common/mod.rs` | harness: dual `dlopen`, stdout/stderr capture, fork-based crash comparison, PRNG, staleness guards, one differential driver per export |
| `tests/configs.rs` | Phase B — 50 tests, one per `CONFIGS.md` row |
| `tests/errors.rs` | Phase C — 36 tests, one per `ERRORS.md` row |
| `tests/symbols.rs` | Phase D — symbol parity and import resolution |
| `tests/smoke.rs` | harness self-test (both libs load; capture really sees library output) |
| `.cargo/config.toml` | `RUST_TEST_THREADS=1` (fd redirection is process-global) + offline registry |

Nothing in `c_src/` was modified. The only additions there are the
`c_src/build/` directory produced by the documented cmake invocation; the `-O2`
cross-check build is written to `translation/target/c_O2/`.
