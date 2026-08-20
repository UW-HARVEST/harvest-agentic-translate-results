# VERIFICATION.md — completion gate

Reproduce everything with `./verify_all.sh` (and `./mutation_check.sh` for the
suite's own sanity check).

## Surface

| item | value |
|---|---|
| C source | `c_src/src/lib.c` (126 lines), `c_src/include/lib.h` (1 line) — one translation unit |
| C functions | `stbds_siphash_bytes` (static), `stbds_hash_bytes`, `siphash` — all three translated |
| exported symbols | `siphash`, `stbds_hash_bytes` — identical in both `.so` files |
| `Cargo.toml [features]` | none → the only feature combination is the empty set |
| `CMakeLists.txt` options / `#ifdef`s | none → one C configuration |

## Test inventory

| binary | tests | covers |
|---|---|---|
| `tests/symbol_parity.rs` | 4 | Phase D — `nm -D` diff, static helper hidden, no unresolved non-libc imports |
| `tests/differential_hash.rs` | 31 | Phase B — `CONFIGS.md` rows 1–30 (+ harness self-check) |
| `tests/differential_siphash.rs` | 1 | Phase B — `CONFIGS.md` rows 31–38 (stdout compared byte-for-byte) |
| `tests/error_paths.rs` | 12 | Phase C — `ERRORS.md` rows 1–10 + 2 extra generic-boundary tests |
| `tests/error_paths_siphash.rs` | 1 | Phase C — `ERRORS.md` row 11 |
| **total** | **49** | |

Every test loads BOTH shared objects with `libloading` and calls only their
exported C symbols; the Rust crate is never linked directly, so the
`#[unsafe(no_mangle)] extern "C"` wrappers are under test too.

## Divergence found and fixed

One real divergence was found, in Phase C row 8:

* **`stbds_hash_bytes(NULL, len > 0, seed)`** — the C object dies from `SIGSEGV`
  (11). The Rust object died from `SIGABRT` (6), because `rustc` injects a
  null-pointer check in front of every raw-pointer dereference under
  `-C debug-assertions`, and `slice::from_raw_parts` additionally carries a
  non-null/alignment precondition assert. Both turn C's hardware fault into a
  Rust abort.
* **Fix (Rust side only):** the byte loads now go through a `load_u8` helper that
  calls libc `memcpy` for one byte, which is exactly the plain load the C
  compiler emits for `d[k]`, with no injected checks. Loading one byte at a time
  (rather than one wide load) also guarantees the translation never touches a
  byte the C code would not — which matters when the input ends exactly at a page
  boundary (`tests/error_paths.rs::boundary_no_overread_past_len`).
* Consequence: the dev and release objects now behave identically to C, and both
  `ERRORS.md` rows 8 and 9 assert the *same signal*, not merely "both failed".

No other divergence was found; in particular the Rust already reproduced all of
the C's signed-overflow / sign-extension quirks (`ERRORS.md` rows 6 and 7), which
are asserted positively — `row06` proves lengths 4..7 *collide* when
`d[3] >= 0x80` and `row07` proves `d[4..8]` are *ignored*, so a Rust that
"fixed" the C's integer promotion would fail.

## Suite is not vacuous

`./mutation_check.sh` injects 18 plausible translation bugs into `src/lib.rs` one
at a time and requires the suite to catch each. Result: **18/18 caught, 0
survivors**, including:

`tail case 4: drop sign extension`, `block low word: drop sign extension`,
`block high word: keep sign extension`, `sipround rotate 13 -> 14`,
`sipround rotate 21 -> 20`, `finalization constant 0xff -> 0xee`,
`len << 56 -> len << 48`, `final rounds 4 -> 3`, `tail case 7 shift 48 -> 56`,
`tail boundary rem >= 4 -> rem > 4`, `loop bound i + WORD <= len -> < len`,
`siphash prints 63 lines`, `siphash mem fill uses init only`,
`siphash printf mask 255 -> 254`, `seed complement: use seed instead of !seed`,
`second seed XOR round removed`, `null deref instead of memcpy load`,
`over-read: load 8 bytes for the tail`.

## Extra robustness: the C's UB is stable

The C code contains signed-overflow UB (`d[3] << 24` with `d[3] >= 0x80`) whose
result the translation depends on. To confirm the observed behaviour is not an
artefact of the unoptimized CMake default, `lib.c` was additionally compiled
(outside `c_src/`, which is left untouched) at `-O0/-O1/-O2/-O3/-Os` and the full
suite was run against each, for both the dev and the release Rust object:
**10/10 combinations pass.**

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing symbols and 0 undefined non-libc
      symbols in the Rust `.so` (asserted at test time by
      `tests/symbol_parity.rs`, and by step 4 of `verify_all.sh` for both the dev
      and release objects). The symbol diff is **empty in both directions**.
- [x] Phase B: all 38 rows of `CONFIGS.md` pass across randomized inputs
      (fixed-seed splitmix64, hundreds to thousands of inputs per row).
- [x] Phase C: all 11 rows of `ERRORS.md` have a passing error-path differential
      test, each asserting the same concrete outcome (same hash value, or the
      same terminating signal), plus 2 extra generic-boundary tests.
- [x] All of the above hold under EVERY feature combination — `default`,
      `--no-default-features` and `--all-features` (all three resolve to the
      empty feature set, since `Cargo.toml` declares no `[features]`), and
      against both the dev and the release `.so`.
