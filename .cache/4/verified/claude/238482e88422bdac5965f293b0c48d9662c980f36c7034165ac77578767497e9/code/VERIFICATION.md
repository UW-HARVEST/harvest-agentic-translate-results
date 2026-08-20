# VERIFICATION.md — differential verification record

Ground truth: the C library in `c_src/`, compiled by its own `CMakeLists.txt`
(`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`, i.e. no `CMAKE_BUILD_TYPE`
→ `-O0`). Nothing in `c_src/` was modified.

Both implementations are exercised **only** through their shared objects, loaded
with `libloading` and called through the exported `md5_digest` symbol — the Rust
side is never called as a Rust function, so the `#[no_mangle] extern "C"`
wrapper is itself under test.

Reproduce everything with:

```
./verify.sh          # C build + feature matrix + symbol parity + all tests, dev & release
./verify.sh --fast   # dev profile only
```

## Phase A — surface maps

| artifact | content |
|----------|---------|
| `SYMBOLS.md` | 1 exported C symbol (`md5_digest`); 0 missing from the Rust `.so`; 0 non-libc undefined symbols |
| `ERRORS.md` | mechanical scan: the C has **no** in-band error surface (no return value, 0 checks, 0 branches). 7 pointer-contract/fault rows (E1–E7) + 3 documented N/A rows (E8–E10) |
| `CONFIGS.md` | 18 rows (C1–C18) over 9 axes derived from the header + the emitted code |

Build-time configuration surface:

* `Cargo.toml` has **no `[features]` table** → exactly **one** feature
  combination exists: the empty set. `verify.sh` derives this mechanically from
  `Cargo.toml` (it enumerates the power set of the `[features]` table) rather
  than assuming it, so it will fan out automatically if features are ever added.
* `c_src/CMakeLists.txt` has no `option()`, no `target_compile_definitions`, no
  `#ifdef` anywhere in the C → no C-side build configuration axis.
* To compensate for the thin feature axis, the whole suite is additionally run
  under **both Cargo profiles** (`dev` and `release`), which turned out to be
  essential — see bug 2 below, which only reproduces in `release`.

## Phase B / C results

| suite | tests | rows covered | dev | release |
|-------|-------|--------------|-----|---------|
| `tests/valid_paths.rs` | 16 | `CONFIGS.md` C1–C12, C15–C18 | PASS | PASS |
| `tests/error_paths.rs` | 10 | `ERRORS.md` E1–E7 (+E8/E9 pinned), `CONFIGS.md` C13–C14 | PASS | PASS |
| `src/lib.rs` unit tests | 3 | struct layout / LE order sanity | PASS | PASS |

Every `CONFIGS.md` row is driven with many seeded-random inputs (splitmix64,
`SEED = 0x5DEECE66D15EA5E5`), not a single hand-picked value: ~20 000 iterations
for the bulk value fuzz (C6), ~20 000 for the combined-axis fuzz (C18: values ×
`m` alignment × `out` alignment × storage class × overlap mode), plus exhaustive
sweeps where the space is small (all 16 byte lanes, all 128 bit positions, all
24 field permutations, all 16 `m`/`out` offsets, all ±1..15 overlaps).

Error-path rows compare more than "both failed": each forks a child per
implementation and compares the **terminating signal** *and* the **bytes
committed to a `MAP_SHARED` output buffer before the fault**, so store order and
the exact fault boundary are verified, not just the fact of a crash.

## Divergences found and fixed (Rust changed; C never touched)

### Bug 1 — NULL pointers aborted (SIGABRT) instead of faulting (SIGSEGV)

* Rows: `ERRORS.md` E1, E2, E3. Profile: `dev` (any build with
  `debug-assertions`).
* Symptom: C died with SIGSEGV(11), Rust died with SIGABRT(6).
* Cause: the translation used `ptr::read_unaligned` and plain place accesses
  (`*p`, `*p = v`). `read_unaligned` carries a *library*-UB
  `assert_unsafe_precondition!` (enabled by `debug-assertions`), and rustc emits
  a codegen-level null check for raw-pointer place derefs, also under
  `debug-assertions`. Either one panics on NULL, and a panic crossing an
  `extern "C"` boundary aborts → SIGABRT.
* Fix: perform the accesses with `ptr::read_volatile` / `ptr::write_volatile`
  (only *language*-UB preconditions, off unless `-Zub-checks`) and use
  `wrapping_add` instead of `add` (which asserts in-bounds address arithmetic).
  The Rust now faults in the load/store itself, exactly like the C.

### Bug 2 — release optimizer narrowed the field load, changing the fault boundary

* Row: `ERRORS.md` E7 (`m` readable for only `k` bytes), `k = 1..3`. Profile:
  `release` only — invisible in `dev`.
* Symptom: with 1 readable byte at `m`, the C committed **0** output bytes
  (its 4-byte load of `m->a` crosses into the guard page and faults first),
  while the Rust committed **1** byte (`0xde`).
* Cause: LLVM narrowed the 4-byte load to the single byte actually consumed by
  `load(a) as u8`, so that load succeeded where the C's faults.
* Fix: make the 16 loads and 16 stores volatile, which forbids narrowing,
  widening, merging, reordering and elimination. The `release` Rust now compiles
  to the same instruction sequence as the C: 16 × (4-byte field load → shift →
  1-byte store), in `out[0]..out[15]` order.

Both bugs are exactly the class the happy path cannot see: every valid-input row
passed before and after the fixes.

## Instruction-level corroboration

```
C (-O0)              Rust (release)
mov (%rdi),%eax      mov (%rdi),%eax
mov %al,(%rsi)       mov %al,(%rsi)
mov (%rdi),%eax      mov (%rdi),%eax
shr $0x8,%eax        mov %ah,0x1(%rsi)
...                  ...
```

16 four-byte loads / 16 one-byte stores in identical order in both.

## Extra robustness check (beyond the required matrix)

The C was additionally built with `-O2` (`-DCMAKE_BUILD_TYPE=Release`, into
`$TMPDIR`, leaving `c_src/` untouched) and both suites were re-run against it via
the `HARVEST_C_SO` override. All 26 differential tests still pass against the
optimized C in both Rust profiles — including E6/E7, even though the `-O2` C
narrows some of its own loads (`movzwl 0x6(%rdi)`, `movzbl 0x7(%rdi)`): those
narrowed lanes sit past the byte where the fault already occurs, so the committed
prefixes still agree. The canonical `-O0` build remains the ground truth.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows **0** missing symbols and 0 non-libc
      undefined symbols in the Rust `.so` (checked for both profiles by
      `verify.sh`).
- [x] Phase B: **all 18** `CONFIGS.md` rows pass across randomized inputs.
- [x] Phase C: **all 7** actionable `ERRORS.md` rows (E1–E7) have passing
      differential tests; E8–E10 are resolved as N/A with a test pinning the
      header so the N/A cannot silently rot.
- [x] All of the above hold under **every** feature combination — there is
      exactly one (no `[features]` table), enumerated mechanically by
      `verify.sh`, and additionally verified under both the `dev` and `release`
      profiles.
- [x] `cargo check --no-default-features --all-targets` clean (0 warnings);
      `cargo clippy --all-targets` clean.

Final `verify.sh` run: **ALL CHECKS PASSED** (29 tests × 2 profiles).
