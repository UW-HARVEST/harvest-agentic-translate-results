# Verification report — C ↔ Rust differential testing of `libdriver`

Scope: `c_src/src/driver.c` + `c_src/include/driver.h` (the entire C library,
114 + 29 lines) against `src/lib.rs`.

Reproduce everything with:

```sh
./run_all_configs.sh          # every feature combination × cargo profile
```

which builds the C `.so` with cmake, builds the Rust `cdylib`, diffs `nm -D`
output, and runs the whole differential suite for each configuration.

## Phase A — surface map

| artifact | content |
|----------|---------|
| `SYMBOLS.md`  | all 5 dynamic symbols of the C `.so`, all present in the Rust `.so`; the 2 `static` C functions stay unexported on both sides; imports are libc-only |
| `ERRORS.md`   | 24 rows — every rejection / boundary / UB trigger found by grepping the C for `if`, `NULL`, `ERROR`, `assert`, `return`, `#if`, constants |
| `CONFIGS.md`  | 29 rows — the full valid-input surface: all 5 entry points (low-level printers included) × input shapes × sequencing / buffering / statelessness / composition axes |

Build-time configurations: `Cargo.toml` has **no `[features]`** and `src/` has no
`#[cfg(feature = …)]`; `CMakeLists.txt` has no options and `driver.c` no `#ifdef`
beyond the header guard ⇒ exactly **one** feature combination (the empty one).
`run_all_configs.sh` still enumerates the combination set generically and runs
each one in both cargo profiles (`dev`, and `release` with `panic = "abort"`).

## Phase B — valid-path differential tests

`tests/valid_paths.rs`, 28 tests, one per `CONFIGS.md` row (row 28 is the
configuration matrix driven by the script).  Both `.so`s are loaded with
`libloading` (`RTLD_NOW | RTLD_LOCAL`) and called only through `dlsym`-resolved
symbols, so the `#[no_mangle]` wrappers are exercised.  Randomised rows use a
fixed-seed splitmix64 PRNG (seeds `0x5EED_00xx`).  Roughly 6 000 differential
calls per profile, including 3 000 random `printIntLine` values, 800 random
`printLine` payloads, exhaustive `bad`/`good` index sweeps, the full
`driver` cross product, and 150 random mixed-call programs.

## Phase C — error-path differential tests

`tests/error_paths.rs`, 24 tests, one per `ERRORS.md` row.  Each constructs the
exact invalid input, runs C and Rust, and asserts the same rejection — the same
message bytes (`ERROR: Array index is negative.` vs
`ERROR: Array index is out-of-bounds`), or no output at all for
`printLine(NULL)` — and the same process termination.  Includes null pointer,
empty/oversized strings, interior pointers, embedded NULs, format-specifier
payloads, `INT_MIN`/`INT_MAX`, one-step-past-range (`9` vs `10`), the missing
upper-bound check in `bad`, and the unreachable `goodG2B` error branch.  This API
declares no enums, so the invalid-enum-variant class degenerates to arbitrary
`int` values, which row 24 covers with the extreme bit patterns.

## Phase D — symbol parity and configurations

`tests/symbols.rs` re-derives the symbol diff from `nm -D` on every run:

```
C .so exports:    {"bad", "driver", "good", "printIntLine", "printLine"}
Rust .so exports: {"bad", "driver", "good", "printIntLine", "printLine"}
missing: {}          undefined non-libc: {}
```

Result of the last full run of `./run_all_configs.sh`:

| features | profile | cargo check | symbol diff | error_paths | valid_paths | symbols |
|----------|---------|-------------|-------------|-------------|-------------|---------|
| `<none>` (= default = all) | `dev`     | ok | 0 missing | 24 passed | 28 passed | 3 passed |
| `<none>` (= default = all) | `release` | ok | 0 missing | 24 passed | 28 passed | 3 passed |

## The one place where byte equality is not required (and why)

`bad(data)` performs `buffer[data] = 1` after checking only `data >= 0`; this is
the injected CWE-129/CWE-787 flaw and is intentionally preserved by the
translation.  For `data >= 10` the C source specifies nothing more than "store 4
bytes at `&buffer[0] + 4*data` on the stack": whether that address is frame
padding, a saved register, a return address or an unmapped page is a property of
the *compiler's* frame layout.  Therefore, for those inputs the tests require

* identical byte streams when both processes survive, and
* prefix-equality when the smashed frame kills one of them,

instead of identical termination.  Measured behaviour: stdout is identical for
every out-of-bounds value tested (`10…64`, plus randomised values up to
`INT_MAX`) in the `dev` profile, and in the `release` profile the only difference
is `driver(_, 12)` / `driver(_, 13)`, where the store hits gcc's saved `rbp`
(harmless until `driver` returns, so gcc emits the final `Finished bad()` line
first) but the return address of the optimised Rust `driver` (which dies one line
earlier).  A slack/padding hack in the Rust `bad` was evaluated and rejected: it
removes those two mismatches but introduces three others (`data` = 14, 15 and 32,
where the C process is the one that dies early), so the faithful, unpadded
translation is the closest match to the C.

Everything in the well-defined input domain — every `printLine`/`printIntLine`
input, `bad(data)` for `data <= 9`, `good(data)` for all `int`, and
`driver(goodData, badData)` for `badData <= 9` — is byte-identical in both cargo
profiles.
