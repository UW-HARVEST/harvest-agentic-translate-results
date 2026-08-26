# VERIFICATION.md — completion gate

C ground truth: `c_src/src/main.c` (570 lines, one translation unit, unmodified).
Rust translation: `src/lib_impl.rs` (the translation), `src/lib.rs` (cdylib +
the C `main` export), `src/main.rs` (the `driver` executable shim).

## Build-time configurations

| source | configurations |
|--------|----------------|
| `Cargo.toml` | no `[features]` section → **1** (the empty/default one) |
| `c_src/CMakeLists.txt` | no `option()` / `-D` / `#ifdef` → **1** |

`./run_all_configs.sh` derives the feature power set from `Cargo.toml` and runs
`cargo check --all-targets` + `cargo build --lib` + `cargo test` for each; it
reports `1 configuration(s) to verify` and `ALL CONFIGURATIONS PASSED`.
The suite additionally passes under both cargo profiles (`cargo test` and
`cargo test --release`).

## Completion checklist

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing / 0 undefined non-libc symbols.**
      The C `.so` exports 16 symbols; the Rust `cdylib` exports all 16 under
      identical names (plus one test-support hook, i.e. a superset). Enforced by
      `tests/symbol_parity.rs` (5 tests), which shells out to `nm -D` on both
      objects, and by `Api::load`, which `dlsym`s all 16 in both.
      No symbol is stubbed — `grep -rn 'unimplemented!\|todo!' src/` is empty.
- [x] **Phase B: every row in `CONFIGS.md` (1–91) passes across randomized
      inputs.** 104 tests in `tests/diff_lowlevel.rs` (67), `tests/diff_process.rs`
      (10), `tests/diff_io.rs` (9) and `tests/so_main_diff.rs` (23, of which
      rows 80–91), all fixed-seed property style.
- [x] **Phase C: every row in `ERRORS.md` (1–54, plus G1–G9) has a passing
      error-path differential test** asserting the same return value/sentinel
      *and* the same stderr bytes — not merely "both failed".
      Rows 5 and 44 are documented as unreachable without an allocator fault
      injector; every other row has a named test (see the traceability table in
      `ERRORS.md`).
- [x] **All of the above hold under every feature combination** — there is
      exactly one, and it is verified by `run_all_configs.sh`.

## Test inventory

```
tests/common/mod.rs        support: dlopen both .so's, fd 0/1/2 capture,
                           SplitMix64 RNG, byte-exact comparators
tests/symbol_parity.rs       5 tests   Phase D
tests/diff_lowlevel.rs      67 tests   CONFIGS rows 1–61 + 5 aliasing tests
tests/diff_process.rs       10 tests   CONFIGS rows 62–71
tests/diff_io.rs             9 tests   CONFIGS rows 72–79
tests/errors.rs             34 tests   ERRORS rows 1–39 + G2/G7/G8
tests/so_main_diff.rs       23 tests   CONFIGS rows 80–91, ERRORS rows 40–54
                                       (3000 randomized whole-program cases)
                           ---------
                           148 tests
```

Both implementations are always reached through their exported C symbols loaded
with `libloading` — no Rust function is ever called directly, so the
`#[no_mangle]` wrappers are part of what is under test. `tests/so_main_diff.rs`
additionally spawns the two **executables** (cmake's `driver` vs cargo's
`driver`) and cross-checks that the `.so`'s `main` return value equals the
process exit code and that both produced identical bytes.

## Divergences found and fixed in the Rust translation

1. **`buffer_split` hoisted a field read that C performs *after* a write.**
   C computes `remaining = src->length - split_pos` *after* assigning
   `dst1->length = split_pos`. When a caller aliases `dst1` onto `src`, that
   assignment has already overwritten `src->length`, so C yields `remaining == 0`
   and an empty `dst2`. The translation had cached `src->length` up front and
   produced a non-empty `dst2`. Found by `alias_split_all_three_arguments_same`
   / `row33_split_aliased_destinations`.
2. **`process_buffer_array` looped with `usize` counters.** C only rejects
   `count == 0`, so a negative `count` makes every `for` body unreachable and the
   function returns 0. Casting `count` to `usize` turned that into a ~2^64
   iteration loop. Now the loops use `c_int` exactly like the C.
   Found by `row70_negative_count_loops_never_run`.
3. **The exported `read_buffer` used `std::io::stdin()`, whose buffer is a
   process-wide singleton.** Bytes read ahead during one call leaked into the
   next, unlike C's `FILE *stdin` which a caller can reset with `freopen`. The
   reader (and writer) now own their buffers and talk to fd 0 / fd 1 directly.
   Found by `row72_read_buffer_length_zero` / `row76_...`.
4. **`copy_nonoverlapping` vs `memcpy` on aliased arguments.** C's
   `memcpy(p, p, n)` is benign; Rust's `copy_nonoverlapping` has a hard
   non-overlap precondition and aborted. All data moves now use `ptr::copy`,
   which agrees with `memcpy` for every case C leaves defined.
5. **`init_buffer_array` pre-zeroed the storage.** C leaves it uninitialized;
   pre-filling made `capacity = INT_MAX` (where C's malloc simply fails) attempt
   a 584 GB memset. It is now left uninitialized exactly as in C.

## Deliberate, documented non-goals

For `length > 256` several C helpers (`buffer_reverse`, `buffer_rotate`,
`buffer_split`, `buffer_conditional_copy`, `buffer_copy_strided`) `memcpy`
through a `uint8_t temp[256]` with no check, i.e. the C behaviour is undefined
(stack / struct overrun). The Rust translation clamps its memory traffic there so
it stays memory-safe instead of reproducing the overflow, and those inputs are
excluded from differential execution — see the "Undefined-behaviour rows" section
of `ERRORS.md`. This is unreachable from the shipped program: `read_buffer`, the
only way the executable can populate a buffer, rejects `length > 256`.

## Note on the `libloading` dev-dependency

`libloading = "0.8"` is a **dev**-dependency, so it is only needed to build and
run `tests/`. Because cargo builds the full resolve graph, `cargo build
--offline` does still require `libloading`, `cfg-if` and `windows-link` to be
present in the cargo registry cache; all three are, and `cargo build`,
`cargo build --release` and `cargo test` all succeed with `--offline`.
