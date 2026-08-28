# VERIFICATION.md — how to reproduce, and what was found

## What is under test

| | |
|---|---|
| C reference | `c_src/src/lib.c` (177 lines, single TU) + `c_src/include/lib.h` |
| C artifact | `c_src/build/lib<parent-dir-name>.so` — 12 exported symbols, compiled at `-O0` (CMake `CMAKE_BUILD_TYPE` is unset) |
| Rust artifact | `translation/target/{debug,release}/libhatch_lib.so` — `crate-type = ["cdylib"]` |
| Test style | 100 % differential through `libloading`. **The Rust crate is never linked or called directly** — both `.so`s are `dlopen`ed and driven through their exported symbols, so the `#[no_mangle] extern "C"` wrappers are themselves under test. |

## Reproduce

```sh
# 1. C reference
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. Rust cdylib + the whole suite, under every feature combo AND both profiles
cd ../../translation && ./run_all_features.sh

# individual pieces
./check_symbols.sh                                     # Phase A / D symbol diff
cargo build --release && cargo test -- --test-threads=1 # Phases B + C + D
./mutation_check.sh                                     # proves the suite bites
```

`--test-threads=1` matters: `lib.c` keeps two `static int`s that every `hatch`
call mutates, so the two libraries must observe the *same sequence* of calls.
The harness also holds a global lock and normalises the hidden state via
`Libs::set_state`, so the suite is correct either way, but single-threaded runs
give deterministic failure attribution.

Runtime: ~6 s for the whole suite on an idle host, ~30 s for
`run_all_features.sh` (5 configurations). The 10 subprocess fault tests dominate
wall-clock time and are sensitive to host load, so on a busy machine the
`error_paths` binary can take up to ~80 s. Each faulting child is exec'd through
`sh -c 'ulimit -c 0; exec ...'` so that a host whose
`/proc/sys/kernel/core_pattern` pipes to `systemd-coredump` neither pays for nor
accumulates 20 core dumps per run.

## The hidden-state problem, and how the harness solves it

`global_counter` / `global_accumulator` (lib.c:29-30) are library-global, never
reset, written by `increment_counter` / `update_accumulator` / `hatch`, and read
by `complex_calc` / `process_pointer_data` / `hatch`. So *the same call with the
same arguments returns different values depending on history*, and a naive test
suite silently desynchronises the two libraries.

The harness makes the hidden state fully observable **and settable** using only
public exports:

| operation | public-API expression | why it works |
|---|---|---|
| read counter | `complex_calc(0,0,0)` | `(0-0)*0 + counter` |
| read accumulator | `process_pointer_data(&v, 0)` | `v*0 + accumulator` |
| write counter | `increment_counter(target - cur, _)` | `counter += delta` |
| write accumulator | `update_accumulator(target - 2*cur, _)` | `acc = acc*2 + value` |

`Libs::set_state(c, a)` drives both libraries to an exact state and cross-checks
the read-back, so every test is order-independent, and the state itself is a
differential assertion on every call.

## Results

| phase | artifact | outcome |
|---|---|---|
| A | `SYMBOLS.md` | 12/12 C symbols exported by the Rust `.so`; symbol diff empty; 0 unresolved non-libc symbols (`ldd -r`) |
| A | `ERRORS.md` | 41 rows mechanically derived (the C has **no** error protocol — only guards, loop bounds, an implicit malloc-failure path and hard faults) |
| A | `CONFIGS.md` | 36 rows across 9 axes |
| B | `tests/valid_paths.rs` | 33 tests, one per `CONFIGS.md` row, ~100 k randomized cases (fixed seeds) — **all pass** |
| C | `tests/error_paths.rs` | 39 tests + 1 subprocess entry point, covering every `ERRORS.md` row, incl. 10 subprocess fault comparisons — **all pass** |
| D | `tests/symbols.rs` + `run_all_features.sh` | symbol diff empty and full suite green under 3 feature combos × 2 build profiles |
| — | `mutation_check.sh` | 25 injected bugs: 23 killed, 2 proven semantically equivalent (must survive) |

Totals: **76 differential tests**, all green (`cargo test -- --test-threads=1`).

## Findings

### 1. FIXED — debug assertions changed observable behaviour (E38)

Found by the build-profile axis (`run_all_features.sh` section 4), not by the
default `cargo test` run.

`lib.c` dereferences caller pointers with no validation, so a `NULL` argument to
`process_pointer_data`, `manipulate_records`, `shift_array_data` or a `NULL`
callee in `apply_operation` is a `SIGSEGV`. With `debug-assertions` on, rustc
injects a `null pointer dereference occurred` check into the raw-pointer loads,
which panics; the panic cannot unwind across `extern "C"`, so the process dies
with **SIGABRT (6) instead of SIGSEGV (11)** — a real, externally visible
divergence from the C for the dev/test profile.

Fix (in `Cargo.toml`, not in the C):

```toml
[profile.dev]
debug-assertions = false
overflow-checks = false
```

After the fix, all four null/bogus-pointer rows report signal 11 from **both**
`.so`s in **both** profiles. All arithmetic already uses explicit `wrapping_*`
calls, so `overflow-checks` is behaviourally irrelevant and is disabled only for
consistency.

### 2. FIXED — the suite could silently test a stale `.so`

`cargo test` does **not** build a `cdylib`-only library target, so the `.so`
under test always comes from a separate `cargo build`. The first version of the
harness picked whichever artifact happened to exist, which meant a leftover
`target/debug/libhatch_lib.so` could be tested while a freshly rebuilt
`target/release` one was ignored — the suite would then report green
*vacuously*. This was caught by `mutation_check.sh`, which suddenly reported
five surviving mutants that it had killed on the previous run.

Fixes: `rust_so_path()` now resolves deterministically
(`$HATCH_RUST_SO` → `target/release` → `target/debug`) and **panics with
`STALE ARTIFACT` if the chosen `.so` is older than `src/lib.rs`**;
`mutation_check.sh` and `run_all_features.sh` both pin `$HATCH_RUST_SO`
explicitly. With the guard in place, all 25 mutants are handled correctly again.

### 3. No behavioural divergence found in the translation logic itself

Every one of the 12 entry points matched byte-for-byte across all
`CONFIGS.md`/`ERRORS.md` rows, including the parts most likely to be
mistranslated, each of which was confirmed to be *load-bearing* by a
corresponding killed mutant:

* signed-overflow wrap-around in all of `+`, `-`, `*` (the C is UB here; `-O0`
  gcc wraps, and the Rust uses `wrapping_*`);
* the `int`→`size_t` widening inside `count * sizeof(int)` (sign extension), and
  the resulting `malloc(~2^64)` → `NULL` path for negative `count`;
* `int`-first then widen in `current_time - (seed * 3600)` — widening before the
  multiply instead of after is a divergence for `|seed| > 596523`
  (mutant `time_widens_before_multiply`, killed);
* the `double`→`int` truncation in `(int)(diff / 100)` — provably in range, so C
  truncation and Rust saturation agree (mutant `time_saturating_cast`, killed);
* the 48-byte `DataRecord` layout and its 0/4/8/16 offsets, asserted against gcc
  and pinned by full post-`memmove` buffer-image comparisons;
* `num_records - shift` recomputed with wrapping every iteration, including the
  pairs where it wraps to a *large positive* bound and the C walks ~100 GiB past
  the buffer (mutant `records_loop_bound_no_wrap`, killed).

### 4. Two mutants are equivalent, and must survive

Documented at the bottom of `mutation_check.sh`:

* `int_to_size_zero_extend` — sign- vs zero-extension in `c_int_to_size` is
  unobservable: the two guarded call sites only ever pass strictly positive
  values, and the third (`compute_with_dynamic_memory` with `count <= 0`) returns
  `0` whether or not the astronomically large `malloc` succeeded.
* `records_guard_le` — relaxing `shift < num_records` to `<=` only adds
  `shift == num_records`, where the extra `memmove` copies 0 bytes. Note the
  *analogous* mutant for `shift_array_data` (`shift_guard_le_size`) **is**
  killed, because there the guard also enables
  `memset(arr + (size - shift_by), 0, shift_by * 4)`, i.e. zeroing the array.

## Deliberately out of scope (stated, not hidden)

* **Thread safety.** `lib.c` mutates two non-atomic `static int`s with no
  locking; the Rust mirrors this with `static mut`. Concurrent `hatch` calls are
  a data race in *both*, so a concurrent test could not have a deterministic
  expected result. The suite serialises access and verifies the sequential
  semantics, which is the only well-defined contract the C offers.
* **`count` near `INT_MAX` in `compute_with_dynamic_memory`.** Whether
  `malloc(count * 4)` (≈ 8 GiB) succeeds depends on the machine's memory state at
  that instant, so the branch taken is a property of the allocator, not of the
  translation. Both libraries import the *same* libc `malloc` (see `SYMBOLS.md`),
  so they follow whichever branch it dictates. Sizes up to 4 MiB, where malloc
  reliably succeeds, are covered by `err_e37_cwdm_large_but_valid_count`.
* **`DataRecord.name` / `.timestamp` contents produced inside `hatch`.**
  `hatch` fills them with `snprintf("Record_%d")` and `time()` but never reads
  them back, and the buffer is `free`d before returning, so they cannot affect
  any observable output. `manipulate_records` *is* tested with randomized
  `name`/`timestamp` bytes and full byte-image comparison, which pins the struct
  layout those fields determine.
* **`malloc` failure inside `hatch`.** `hatch` does not check its three
  `malloc`s; forcing a failure would need an allocator interposer and both
  libraries would then fault identically.
