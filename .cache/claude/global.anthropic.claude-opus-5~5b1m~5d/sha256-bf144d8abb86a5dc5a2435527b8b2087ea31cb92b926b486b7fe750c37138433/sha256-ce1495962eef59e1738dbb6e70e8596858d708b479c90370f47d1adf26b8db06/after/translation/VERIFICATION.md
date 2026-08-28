# VERIFICATION.md — completion gate

Reproduce everything with:

```sh
cd translation
./verify.sh           # builds C .so + Rust cdylib (all profiles/features), runs the suite
./mutation_check.sh   # proves the suite actually catches divergence
```

## Result

**The Rust translation matches the C ground truth on every input tested. No
divergence was found, so `src/lib.rs` needed no behavioural fix.**

## The library under verification

The entire C library is one function, `char *tool_basename(char *path)`
(22 lines in `c_src/src/lib.c`). It returns a pointer to the final path
component, treating both `'/'` and `'\\'` as separators and preferring whichever
occurs *later* in the string.

## Completion checklist

- [x] **`cargo check` passes** — no compile errors (it was already clean; the
      only edit to `Cargo.toml` was adding `libloading` to `[dev-dependencies]`).
- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing and 0 undefined non-libc symbols
      in the Rust `.so`.** The C `.so` exports exactly one symbol,
      `tool_basename`; the Rust `.so` exports it under the identical name. The
      symbol diff is **empty**, and no C module was left untranslated (the
      library is a single file, fully covered). Machine-checked by
      `tests/phase_d_symbols.rs` and again by `verify.sh` per configuration.
- [x] **Phase B: every one of the 20 rows in `CONFIGS.md` passes** across
      randomized inputs (fixed seeds, ~40 000 generated inputs total plus
      exhaustive sweeps of all 256 byte values and of lengths 2..=64).
- [x] **Phase C: every one of the 10 rows in `ERRORS.md` has a passing
      error-path differential test**, plus 3 extra generic-boundary tests.
- [x] **All of the above hold under every feature combination.** `Cargo.toml`
      has no `[features]` section, so the only combination that exists is the
      default (empty) one — `verify.sh` derives this mechanically from
      `Cargo.toml` rather than assuming it, and would expand to the full
      power set if a feature were ever added. Verified in **both** the `dev` and
      `release` profiles, which matters here because `[profile.release]` sets
      `panic = "abort"`.

## Test inventory

| file | tests | covers |
|---|---|---|
| `tests/common/mod.rs` | — | `libloading` harness, deterministic PCG32 RNG, staleness guard |
| `tests/phase_b_configs.rs` | 20 | `CONFIGS.md` rows C1–C20 |
| `tests/phase_c_errors.rs` | 12 | `ERRORS.md` rows E2–E10 + 3 generic-boundary tests |
| `tests/phase_c_null_ptr.rs` | 1 (+1 ignored helper) | `ERRORS.md` row E1 (NULL, via subprocess signal comparison) |
| `tests/phase_d_symbols.rs` | 3 | symbol parity, dlsym callability, no stray exports |
| **total** | **36 passing** | |

## How the comparison is done

Both implementations are loaded as **shared objects** through `libloading` and
called across the FFI boundary; the Rust function is never called directly, so
the `#[no_mangle] extern "C"` export wrapper is itself under test. For every
input, four things are compared byte-for-byte:

1. the **offset** of the returned interior pointer from the buffer base (the
   complete information content of the return value, since the result always
   aliases the caller's buffer);
2. the returned **C string bytes**;
3. **NULL-ness** (the C never returns NULL; the Rust must not either);
4. the **input buffer after the call** — neither may mutate it.

Each implementation gets its own copy of the buffer, so neither can hide a
difference by mutating shared state. Row C19 additionally passes one shared
buffer to C → Rust → C to check call-order independence.

## Two findings about the harness itself

Both were fixed; neither was a bug in the translated logic.

1. **`cargo test` does not rebuild a `cdylib`-only lib target.** Verified
   directly: touching `src/lib.rs` and running `cargo test` left
   `target/debug/libdriver.so` untouched. Without protection, the entire suite
   could pass green against a stale artifact. `tests/common/mod.rs` now aborts
   with a `STALE ARTIFACT` panic if any file in `src/` or `Cargo.toml` is newer
   than the `.so`, and `verify.sh` always builds before testing.
2. **`[profile.release] panic = "abort"`** prevents `cargo test --release` from
   working (libtest needs unwinding). `verify.sh` therefore keeps the harness on
   the dev profile and points it at the release `cdylib` via `DRIVER_RUST_SO`,
   so the release artifact is genuinely exercised.

## Mutation testing (evidence the suite is discriminating)

Passing tests only mean something if they can fail. `mutation_check.sh` injects
13 plausible translation bugs into `src/lib.rs`, rebuilds, and requires the
suite to fail. **All 13 are caught:**

| mutation | caught |
|---|---|
| last-match → first-match (`rposition` → `position`) | ✅ |
| ternary arm flipped (`i1 > i2` → `i1 < i2`) | ✅ |
| ternary arm dropped (always prefer `'/'`) | ✅ |
| no-separator case returns `NULL` | ✅ |
| off-by-one: `+1` dropped in the both-present arm | ✅ |
| off-by-one: `+1` dropped in the slash-only arm | ✅ |
| `'\\'` no longer treated as a separator | ✅ |
| `'/'` no longer treated as a separator | ✅ |
| `0x80` wrongly matched as a separator | ✅ |
| search stops at the first non-ASCII byte (UTF-8 trap) | ✅ |
| byte compare widened to signed `char` | ✅ |
| input buffer mutated (separator NUL-ed out) | ✅ |
| `#[no_mangle]` export removed | ✅ |

## Notes on faithfulness to the C

* The C's `s1 > s2` is a **pointer** comparison between two positions in one
  buffer; the Rust compares the equivalent indices. Equality is impossible (a
  single byte cannot be both separators), so the ternary's tie case is
  unreachable in both.
* A trailing separator makes the C return a pointer to the **NUL terminator**
  (i.e. `""`), which is a valid in-bounds pointer, not an error. The Rust does
  the same; rows C10, C20, E4, E5 pin this down.
* The C never validates encoding. Rows C16 and E9 feed invalid UTF-8, including
  overlong encodings of `'/'` (`\xc0\xaf`) and `'\\'` (`\xc1\x9c`), confirming
  neither implementation decodes them into separators.
* `char` is signed on x86-64 while `strrchr` compares as `unsigned char`. Rows
  C15 and E7 sweep all of `0x80..=0xFF`, including `0xAF` (`'/' | 0x80`) and
  `0xDC` (`'\\' | 0x80`).
* `path == NULL` is undefined behaviour in the C (no NULL check before
  `strrchr`). Row E1 confirms both implementations die on **SIGSEGV (11)**,
  compared across two child processes so the harness survives.
* There is **no** enum or integer parameter anywhere in the ABI, so the
  "out-of-range enum value across FFI" class of bug cannot be constructed here.
  `phase_c_errors.rs::generic_no_enum_or_integer_parameter_exists` asserts the
  public header still reads exactly `char *tool_basename(char *path);`, so this
  reasoning cannot go stale unnoticed.
