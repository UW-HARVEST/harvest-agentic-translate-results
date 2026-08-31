# VERIFICATION.md — jansson 2.15.0 C → Rust differential verification

Every test loads BOTH shared objects through `libloading` and compares their
behaviour across the FFI boundary. No Rust function is ever called directly, so
the `#[no_mangle]`/`extern "C"` export wrappers are themselves under test.

- C ground truth: `c_src/build/libjansson.so` (CMake, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
- Under test: `translation/target/release/libjansson.so` (`cdylib`, `panic = "abort"`)
- Reproduce everything: `./run_tests.sh` — rebuilds both libraries and the
  `va_list` shim, checks `nm -D` symbol parity, then runs all 16 suites.
- Progress against the gate: `./status.sh`

## Completion gate

| # | criterion | result |
|---|-----------|--------|
| 1 | `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust | **PASS** — 130/130 exported, 0 missing, 0 extra, 0 undefined non-libc |
| 2 | Phase B: every `CONFIGS.md` row passes across randomized inputs | **PASS** — 406 verified + 1 documented-unreachable = 407/407 |
| 3 | Phase C: every `ERRORS.md` row has a passing error-path test | **PASS** — 338 verified + 18 documented-unreachable = 356/356 |
| 4 | All of the above under EVERY feature combination | **PASS** — `Cargo.toml` declares no `[features]`, so the default build is the only configuration; verified under it |

**398 tests across 16 binaries, 0 failures.**

## Test suites

| suite | tests | scope |
|---|---:|---|
| `a00_smoke` | 4 | both `.so`s load; all 130 symbols resolve; version; `dtoa_divmax` |
| `a01_utf` | 11 | `utf8_*` — EXHAUSTIVE over its domains |
| `a02_strbuffer_memory_error` | 25 | `strbuffer.c`, `memory.c`, `error.c` |
| `a03_hashtable` | 18 | `hashtable.c`, `hashtable_seed.c` — full iteration-order comparison |
| `a04_value` | 69 | `value.c` — Phase B |
| `a05_strconv_dtoa` | 27 | `strconv.c` + exported `dtoa.c` entry points |
| `a06_dump` | 23 | `dump.c` — Phase B, full flag cross-product |
| `a07_load` | 22 | `load.c` — Phase B, all six entry points |
| `a08_pack_unpack` | 71 | `pack_unpack.c` — Phase B, every format char and modifier |
| `a09_errors_lowlevel` | 16 | Phase C — low-level modules |
| `a10_lowlevel_gaps` | 9 | Phase C — residual low-level rows |
| `a11_errors_load` | 25 | Phase C — `load.c` |
| `a12_errors_dump` | 15 | Phase C — `dump.c` |
| `a13_errors_value` | 24 | Phase C — `value.c` |
| `a14_errors_pack_unpack` | 32 | Phase C — `pack_unpack.c` |
| `a15_errors_unreachable` | 7 | the rows that cannot be reached in-process, plus their guards |

## Bugs found and fixed in the Rust translation

Both were fixed in `translation/src/`; `c_src/` was never modified.

### 1. `src/dtoa.rs` — `gethex` underflow rounding had its sign tests inverted

The C, for a hex float whose binary exponent underflows (`big && esign`):

```c
case Round_up:   if (sign) break;  goto ret_tiny;   /* sign == 0 -> smallest denormal */
case Round_down: if (!sign) break; goto ret_tiny;   /* sign != 0 -> smallest denormal */
```

The Rust had both conditions the other way round, so `gethex("0x1p-999999999999",
Round_up, sign=0)` produced `0.0` where the C produces the smallest denormal
(bit pattern `1`). Caught by `a09_errors_lowlevel::gethex_overflow_underflow_and_no_digit_paths`
(ERRORS.md rows 329/331).

### 2. `src/dump.rs` — `do_dump` unwrapped the callback before the NULL check

The Rust did `let dumpf = dump.unwrap();` as its first statement. The C answers
`json_dump_callback(NULL, NULL, d, JSON_ENCODE_ANY)` with `-1` from
`if (!json) return -1;` without ever touching the callback, so the Rust panicked
— and with `panic = "abort"` that killed the process. The null check and the
type-tag validation now run first, matching the C on both the `!json` path and
the `default:` arm. Caught by `a12_errors_dump` (ERRORS.md rows 200, 220).

## Why these tests are not vacuous

Rather than trusting green output, each Phase B/C area was validated by
temporarily injecting a bug into the Rust, confirming failures, then reverting:

- `dump.rs`: changing the `/` escape to `\?` → 10 of 23 tests failed.
- `value.rs`: 8 separate mutations (dropped `utf8_check_string`, dropped the
  `json == value` self-insertion guard, `>` → `>=` on an index bound, a removed
  `json_decref`, a `default:` arm returning the input, a broken loop check, a
  dropped type guard, `||` → `&&` in a NULL check) → all 8 caught.
- `load.rs`: flipping one decode flag bit → 7 tests failed.
- `pack_unpack.rs`: two rounds (wrong error code; `+1` on the reported column;
  a changed `source` string) → 7 and then 27 of 32 tests failed, proving the
  code-byte, line/column/position and `source` channels are all load-bearing.

## Harness design notes

- **Deterministic hash seed.** `both()` calls `json_object_seed(FIXED_SEED)` on
  both libraries before anything else. jansson otherwise seeds `hashlittle` from
  `/dev/urandom`, and the seed decides object iteration order and therefore the
  exact bytes of every object dump.
- **Comparison strength.** Errors are compared as the FULL 252-byte
  `json_error_t` image from a poisoned start, which pins line, column, position,
  `source`, `text` and the code byte at once. Strings are compared as raw bytes
  (`cbytes`) because the `_nocheck` entry points can emit non-UTF-8. Doubles are
  compared as bit patterns so `-0.0` stays distinct from `0.0`.
- **`va_list` entry points.** `json_vpack_ex`, `json_vunpack_ex`,
  `json_vsprintf` and `jsonp_error_vset` are driven through a small C shim
  (`tests/vashim.c`, compiled to its own `.so`) that turns a variadic call into a
  real `va_list`; `a08` additionally hand-builds an x86-64 SysV `va_list` to get
  run-time arity.
- **OOM paths.** Failing and *budgeted* allocators installed via
  `json_set_alloc_funcs2` make the out-of-memory rows reachable. Sweeping the
  budget and requiring both libraries to fail at the SAME allocation index also
  proves they perform the identical allocation sequence.
- **Mandatory `global_state_lock()`.** Two pieces of C state are process-global
  and not thread-safe: dtoa's `Balloc`/`Bfree` freelist (`dtoa.c` is compiled
  without `MULTIPLE_THREADS`) and the allocator function pointers. Without
  serialisation the **C** side returns plausible-but-wrong digits under parallel
  test threads, which reads exactly like a translation bug. See
  `tests/HARNESS.md`.
- **Staleness guard.** `cargo test --test <name>` does not rebuild the `cdylib`
  (the test `dlopen`s it rather than linking it), so the harness refuses to run
  if any `src/*.rs` is newer than the `.so`.

## Rows marked `[-]` (documented-unreachable, not verified)

19 rows across the two tables describe conditions that are undefined behaviour
in the C itself, abort via a live `assert()` (the build passes no `-DNDEBUG`), or
are dead code. A differential test could only show that both implementations
crash, which proves nothing about equivalence, so instead each is documented with
the relevant C quoted, and the *guard* that makes it unreachable is verified
through the FFI. Examples:

- `CONFIGS` 309 / `ERRORS` 297 — `hashtable_set`'s key-length overflow guard is
  dead code: `hash_str` reads the key before the guard runs, so any `key_len`
  large enough to trip it segfaults in `hashlittle` first.
- `ERRORS` 314 — `jsonp_strtod`'s `assert(end == value + length)`; the guard
  (every caller passes a fully-consumed buffer) is instead verified over ~4000
  lexer-accepted numeric literals.
- `ERRORS` 323/325 — `Balloc` never checks `MALLOC`, and `freedtoa(NULL)`
  dereferences `((int*)s - 1)`.
- `ERRORS` 342/346/349 — internal lexer invariants; the first `lex_scan_string`
  pass has already validated every escape, which is verified exhaustively.

Each such row carries its reason inline in `CONFIGS.md` / `ERRORS.md`.
