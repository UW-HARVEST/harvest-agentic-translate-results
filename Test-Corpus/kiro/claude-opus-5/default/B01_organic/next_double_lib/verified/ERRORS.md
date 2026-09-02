# ERRORS.md — Error-surface table (Phase A, gates Phase C)

Derived **mechanically** from the C source, not from documentation or
assumption. The exhaustive grep over `c_src/include` + `c_src/src`:

```
$ grep -nE 'RETURN_ERROR|return *-1|return *NULL|assert|errno|ERROR|_ERR|if *\(|switch|#if|<=|>=|== *0|!= *0|MAX|MIN|goto' -r include src
(none found)

$ grep -nE 'return' -r include src
src/lib.c:11:    return x + y;
src/lib.c:19:    return *(double *)&result - 1.0;
```

## Finding: the C library has NO explicit error surface

Both C functions are **totally branch-free**. There is:

* no error-return macro, no `return -1`, no `return NULL`, no error enum;
* no `assert` (and no `<assert.h>`);
* no range check, no null check, no size/length parameter at all;
* no `if`, no `switch`, no `#if`/`#ifdef` — the object code has zero conditional
  jumps derived from input;
* no min/max constant, no magic sentinel;
* no enum type anywhere in the public header, so there is no
  out-of-range-enum-value class of input;
* the only pointer parameter, `cn_rnd_t *rnd`, is dereferenced unconditionally
  (`c_src/src/lib.c:4`) with no guard.

`double next_double(cn_rnd_t *)` is a total function over its 128-bit input
state: **every** one of the 2^128 possible `cn_rnd_t` values is valid input and
produces a defined `double` in `[0.0, 1.0)`. There is therefore no "invalid
value" for the state words, and the table below has no value-triggered rows.

## The table

One row per distinct way the C code rejects or errors on input.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| — | — | *(no explicit rejection exists anywhere in the C source — see grep above)* | — | `errors::e00_no_explicit_error_surface_in_c` (asserts the grep-derived fact structurally: all 2^64-spanning state classes return a finite value in `[0,1)` from both libs, i.e. nothing is ever rejected) |

Row count of genuine, C-implemented rejections: **0**.

Per the task instructions, a stub or invented error row would be worse than an
empty table, so no rows are fabricated. The generic FFI boundary conditions that
every C API has are still covered below.

## Generic C-API boundary conditions (covered even though not in the table)

These are required by Phase C regardless of the table being empty. Each has a
differential test that drives **both** `.so`s and compares behaviour.

| # | condition | why it is/isn't applicable here | C behaviour | Rust must match | test |
|---|-----------|----------------------------------|-------------|-----------------|------|
| B1 | `rnd == NULL` | applicable: unguarded deref at `src/lib.c:4` | UB in practice `SIGSEGV` (signal 11, fault on load of `rnd->state[0]`); no error code, no return | same fatal signal (11), same absence of any error return | `errors::b1_null_pointer_same_fatal_signal` (both calls made in forked child processes; asserts identical termination signal **and** identical exit code, and that neither returns normally) |
| B2 | zero length / oversized length | **not applicable**: `next_double` takes no length, size, or count parameter | n/a | n/a | `errors::b2_no_length_parameter_exists` (documents + asserts the signature has exactly one parameter by calling with the single 16-byte struct) |
| B3 | value one step past a documented valid range | **not applicable to the state words**: the valid range of each `uint64_t` state word is the whole `uint64_t` domain; there is no documented sub-range, hence no "one past". Both extremes and their neighbours are tested as *valid* input instead. | defined result for `0`, `1`, `u64::MAX-1`, `u64::MAX` | identical bits | `errors::b3_extremes_and_neighbours_are_valid` |
| B4 | out-of-range enum value across the FFI boundary | **not applicable**: the public header declares no enum, and `next_double` takes no `int`-like parameter that could carry an invalid discriminant | n/a | n/a | `errors::b4_no_enum_in_public_api` (documents the absence; the only parameter is a struct pointer) |
| B5 | mis-sized / partially-initialised struct | applicable as ABI check: caller supplies a raw 16-byte buffer | reads exactly `state[0]` and `state[1]`, writes exactly `state[0]` and `state[1]`; touches no adjacent bytes | identical, incl. not disturbing guard bytes around the struct | `errors::b5_no_out_of_bounds_struct_access` (red-zone guard bytes either side) |
| B6 | unaligned / heap vs stack struct address | applicable as ABI check | same result for any properly aligned address | identical | `errors::b6_heap_and_stack_addresses_agree` |
| B7 | repeated calls after the degenerate all-zero state | applicable: `(0,0)` is a fixed point (no error, but the one input class that never changes state) | returns `0.0` forever, state stays `(0,0)` | identical | `errors::b7_all_zero_state_is_a_fixed_point` |

## Checklist (Phase C gate)

* [x] Row `—` (empty table — no explicit rejection in C): verified by grep + structural test
* [x] B1 null pointer
* [x] B2 no length parameter
* [x] B3 range extremes and their neighbours
* [x] B4 no out-of-range enum possible
* [x] B5 no out-of-bounds struct access
* [x] B6 heap vs stack address
* [x] B7 all-zero fixed point

## Divergence found and fixed (Phase C)

**B1 — NULL pointer: Rust aborted where C segfaulted.**

The original translation did `let rnd: &mut cn_rnd_t = unsafe { &mut *rnd };`.
Forming a Rust reference from the pointer trips a debug-build UB check, so the
debug-profile `cdylib` terminated with **SIGABRT (6)** while the C library
terminated with **SIGSEGV (11)** — an observable behavioural difference for the
same input, on the exact input class this row covers. (The release-profile
`cdylib` happened to segfault, so a release-only test run would have missed it.)

Fix, in `src/lib.rs` (the Rust was changed; the C was not): read and write the
two state words through `core::ptr::read` / `core::ptr::write` on a `*mut u64`
instead of materialising a `&mut`. A raw pointer read faults exactly the way the
C load does, so both libraries now die with SIGSEGV in **every** profile.
Verified empirically that `&mut *p` aborts while `ptr::read(p)` segfaults under
`debug-assertions`.

## Evidence

```
$ ./run_all.sh
  OK    tests (36 passed)  combo=default profile=dev     cdylib=debug
  OK    tests (36 passed)  combo=default profile=dev     cdylib=release
  OK    tests (36 passed)  combo=default profile=release cdylib=debug
  OK    tests (36 passed)  combo=default profile=release cdylib=release
  OK    tests (36 passed)  combo=none    profile=dev     cdylib=debug
  OK    tests (36 passed)  combo=none    profile=dev     cdylib=release
  OK    tests (36 passed)  combo=none    profile=release cdylib=debug
  OK    tests (36 passed)  combo=none    profile=release cdylib=release
ALL CHECKS PASSED
```

Additionally, because `next_double` type-puns through `*(double *)&result`
(a strict-aliasing violation that a compiler is free to exploit), the whole
suite was re-run against the C library rebuilt at `-O0`, `-O1`, `-O2` and `-O3`.
All 36 tests pass against every one, so the Rust matches the C's behaviour
independently of how the C was optimised.
