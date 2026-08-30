# ERRORS.md — the error-surface table (Phase A / Phase C)

Derived mechanically from the C sources, not from docs. The grep used:

```sh
grep -nE 'return|assert|NULL|-1|exit|errno|if *\(|switch|case|default' \
     c_src/src/mdcore.c c_src/src/mdmain.c c_src/src/mdmacros.h
```

## What the grep found (complete)

The library has an unusually small error surface, and it is important to record
that *mechanically* rather than assume it:

* **No `RETURN_ERROR`-style macro, no error enum, no `-1`/`NULL` sentinel, no
  `errno` use, no `exit()`, no `assert()`** anywhere in `mdcore.c`,
  `mdmacros.h`, or `mdmain.c`.
* **No pointer parameters at all.** Every exported function takes only `int` and
  returns `int`, so there is no null-pointer or length check to make. The only
  pointers in the library are the two globals `G_OP` / `G_OP_NAME`, which are
  *outputs* the consumer reads, never inputs the library validates.
* The only *runtime* rejection is the `default:` arm of `DISPATCH_REP`
  (`mdmacros.h:91`), reached through `use_generated`.
* The only *process-level* rejection is `argc < 3` in `main` (`mdmain.c:29-32`).
* The remaining rejections are **build-time**: the `#ifndef OP` / `#ifndef REPEAT`
  fallbacks (`mdmacros.h:27-32`) and the token-paste failures for out-of-range
  `OP` / `REPEAT` values (`CAT` at `mdmacros.h:36`).

Because `int` parameters accept every bit pattern, the "out-of-range enum value
across the FFI boundary" class of bug shows up here as **out-of-range `n` passed
to `use_generated`**: `n` selects a `switch` arm, so it is an enum-like selector
in all but name, and every `n` outside `0..=6` must silently take `default:`.
Rows 1–8 below cover that exhaustively.

## The table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `use_generated` | `n == 7` — one step past the last `case` (`case 6`) in `DISPATCH_REP`; note `REP7` *exists* as a macro but is **not** wired into the `switch`, so 7 is rejected even though `REPEAT=7` is a legal build | `default: break` taken ⇒ `acc` keeps `INIT_FOR(OP)`; returns `0` for `add`/`sub`, `1` for `mul`. Prints `gen.acc=<that>` |
| 2 | `use_generated` | `n == -1` — one step below the first `case` (`case 0`) | `default:` ⇒ returns `INIT_FOR(OP)` (`0`/`0`/`1`) |
| 3 | `use_generated` | `n == INT_MIN` — extreme negative | `default:` ⇒ returns `INIT_FOR(OP)` |
| 4 | `use_generated` | `n == INT_MAX` — extreme positive | `default:` ⇒ returns `INIT_FOR(OP)` |
| 5 | `use_generated` | `n == 8` — first value beyond any defined `REPn` | `default:` ⇒ returns `INIT_FOR(OP)` |
| 6 | `use_generated` | any `n` in `[-2^31, -1]` (randomised sweep of the whole negative half) | `default:` ⇒ returns `INIT_FOR(OP)` for every one |
| 7 | `use_generated` | any `n` in `[7, 2^31)` (randomised sweep of the whole out-of-range positive tail) | `default:` ⇒ returns `INIT_FOR(OP)` for every one |
| 8 | `use_generated` | `n` such that only the low bits are in range, e.g. `0x1_0000_0000 + 3` truncated to `int` by the ABI ⇒ `3` | the ABI truncates to 32 bits *before* the `switch`, so this is the **valid** `case 3`, not `default:` — Rust must truncate identically rather than reject |
| 9 | `op_add` | `a + b` overflows `int` (e.g. `INT_MAX + 1`, `INT_MIN + (-1)`) — signed overflow is UB in C, but the compiled `add` instruction wraps two's-complement | wraps; must match the C `.so` bit-for-bit (Rust uses `wrapping_add`) |
| 10 | `op_sub` | `a - b` overflows `int` (e.g. `INT_MIN - 1`, `INT_MAX - (-1)`) | wraps two's-complement (Rust `wrapping_sub`) |
| 11 | `op_mul` | `a * b` overflows `int` (e.g. `INT_MAX * INT_MAX`, `INT_MIN * -1`) | wraps two's-complement (Rust `wrapping_mul`) |
| 12 | `helper_call` | `r + acc` overflows `int` in the `return r + acc` (reachable with `a`/`b` near `INT_MAX`) | wraps two's-complement |
| 13 | `helper_call` | `mul` build: the unrolled `acc *= (i+1)` chain overflows for large `REPEAT` (`REPEAT=7` ⇒ `7! = 5040`, no overflow; overflow only via the `r + acc` sum) — and `sub` build: `acc -= i` starting from `0` goes negative | wraps / stays negative; no rejection, `acc` is simply the wrapped value |
| 14 | `helper_ptr` | any `a`,`b` including overflowing pairs; `fp` is `OP_FN(OP)` and can never be null (it is a compile-time constant), so there is **no** null-function-pointer path to test | delegates to the selected `op_*`, wraps like rows 9–11 |
| 15 | `G_OP` (global) | consumer writes a new value through the `dlsym` address — the object is in writable `.data` (`nm` type `D`), so the store must **succeed**, not fault | store succeeds; `helper_ptr`/`helper_call` are unaffected because they use `OP_FN(OP)`, not `G_OP` |
| 16 | `G_OP` (global) | consumer stores a **null** function pointer into `G_OP` and then calls it | the *library* never dereferences `G_OP` (only `mdmain.c`'s `main` does), so the library exposes no crash; parity is "the store is accepted and the library keeps working" |
| 17 | `G_OP_NAME` (global) | consumer writes through the `dlsym` address — also writable `.data` | store succeeds (same as row 15) |
| 18 | `main` (`driver`) | `argc < 3`, i.e. invoked with 0 or 1 positional argument | writes `usage: <argv[0]> A B\n` to **stderr** (not stdout) and returns exit status **2**; produces no stdout |
| 19 | `main` (`driver`) | argument that is not a number, e.g. `"abc"` — `atoi` has no error report | `atoi` returns `0`; no rejection |
| 20 | `main` (`driver`) | argument that overflows `long`, e.g. `"99999999999999999999"` | `atoi` is `(int)strtol(...)`; `strtol` clamps to `LONG_MAX`/`LONG_MIN` then the cast truncates ⇒ `-1` / `0` respectively. No rejection |
| 21 | `main` (`driver`) | extra arguments beyond `A B` (`argc > 3`) | silently ignored; not an error |
| 22 | build-time | `OP` undefined (`-DOP` omitted) — `#ifndef OP` at `mdmacros.h:27` | defaults to `add`. Mirrored by "no OP feature ⇒ `add`" |
| 23 | build-time | `REPEAT` undefined — `#ifndef REPEAT` at `mdmacros.h:30` | defaults to `5`. Mirrored by "no REPEAT feature ⇒ `5`" |
| 24 | build-time | `OP` not in `{add, sub, mul}`, e.g. `-DOP=div` | **compile error**: `'op_div' undeclared`, `'INIT_div' undeclared` (verified). Not expressible in Cargo — the feature set is exactly `add`/`sub`/`mul` |
| 26 | `main` (`driver`) | `argc == 0` — `execve`'d with an empty `argv` array, so `argv[0]` is **NULL** and that null pointer is passed to `fprintf`'s `%s`. Not reachable via `std::process::Command`; the test re-`execve`s from `pre_exec` | glibc renders the null `%s` as the **empty string**, so stderr is `"usage:  A B\n"` (two spaces), stdout empty, exit status `2`. Verified empirically — *not* `"(null)"` |
| 25 | build-time | `REPEAT` not in `0..=7`, e.g. `-DREPEAT=8` or `-DREPEAT=-1` | **compile error**: `REP8` undeclared / `pasting "REP" and "-" does not give a valid preprocessing token` (verified). Not expressible in Cargo — the feature set is exactly `0`..`7` |

## Where each row is discharged

| rows | test | status |
|------|------|--------|
| 1–14 | `tests/errors.rs` — through the `.so` FFI boundary (`dlopen`/`dlsym`) | [x] passing |
| 15–17 | `tests/globals.rs` — own process, because it overwrites the process-global `.data` objects | [x] passing |
| 18–21, 26 | `tests/driver_cli.rs` — spawns both `driver` executables, compares stdout + stderr + exit status | [x] passing |
| 22–23 | `check_all_features.sh` — the "no features" combination must behave as `add`/`5` (also `CONFIGS.md` row 25, `run_all_configs.sh` config 25) | [x] passing |
| 24–25 | not representable in Cargo: the feature set is exactly `add`/`sub`/`mul` and `0`..`7`, so an out-of-range `OP`/`REPEAT` cannot be requested. Verified as a *compile error* on the C side with `gcc -fsyntax-only -DOP=div` and `-DREPEAT=8` / `-DREPEAT=-1` | [x] verified |

**All 26 rows discharged.** Every row-1–21/26 test is run once per configuration
by `run_all_configs.sh` (41 configurations, dev and release profiles).

## Notes on the two easy-to-miss classes

* **Over-eager rejection.** Row 8 is the mirror image of rows 1–7: because the
  `switch` selector arrives as a 32-bit `int`, a caller passing a wider value
  reaches a *valid* `case` after ABI truncation. A Rust translation that
  range-checked before truncating would wrongly take the `default:` arm.
* **Null across the boundary, in the one place it can happen.** The library has no
  pointer *parameters*, so the only null pointer the C code can be handed is
  `argv[0]` when `argc == 0` (row 26). It is worth testing precisely because it is
  the sole instance of the class, and because the naive Rust translation
  (`args.first()` ⇒ `None`) happens to agree with glibc only by coincidence.
* **Read-only vs. writable globals.** Rows 15–17 are not about invalid *input* at
  all but about the *storage class* of the two exported data symbols. This was a
  real divergence: an immutable Rust `static` holding a relocated function
  pointer is emitted into `.data.rel.ro` and becomes read-only after RELRO
  processing, so a consumer storing through the `dlsym` address would fault where
  the C library (`nm` type `D`, section `.data`) succeeds. `G_OP`/`G_OP_NAME` are
  therefore `static mut`, and `readelf -S` confirms both now land in `.data`.
