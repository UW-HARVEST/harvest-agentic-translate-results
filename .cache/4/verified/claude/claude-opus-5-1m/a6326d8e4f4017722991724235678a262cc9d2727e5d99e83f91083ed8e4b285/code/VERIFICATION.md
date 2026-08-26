# VERIFICATION.md — completion gate

Run everything with:

```sh
./run_tests.sh          # builds C .so, builds Rust cdylib, runs all suites
```

## Completion checklist

- [x] **`SYMBOLS.md`** — `nm -D` shows **0** missing/undefined non-libc symbols in
      Rust. The C `.so` exports exactly one symbol (`crc16`); the Rust `.so`
      exports it under the same name. Symbol diff is **empty** in both
      directions (ignoring Rust std-runtime symbols). Enforced automatically by
      `tests/symbol_parity.rs`, not just by hand.
- [x] **Phase B** — every one of the **20 rows** in `CONFIGS.md` passes across
      randomized inputs (fixed seed `0x2545F4914F6CDD1D`), including two
      exhaustive sweeps (all 65 536 seeds, all 256 byte values).
- [x] **Phase C** — every one of the **8 rows** in `ERRORS.md` has a passing
      error-path differential test, plus 3 generic-boundary tests.
- [x] **All feature combinations** — `Cargo.toml` has no `[features]` section and
      `c_src/CMakeLists.txt` has no options or `#ifdef`s, so the complete matrix
      is a single empty-feature configuration. It was additionally verified
      against the release profile (`panic = "abort"`) and against the C compiled
      at `-O0`/`-O2`/`-O3`.

## Test inventory — 35 tests, 0 failures

| suite | tests | covers |
|---|---|---|
| `tests/differential_valid.rs` | 20 | `CONFIGS.md` rows C1–C20 |
| `tests/differential_errors.rs` | 11 | `ERRORS.md` rows E1–E8 + 3 generic FFI boundary tests |
| `tests/symbol_parity.rs` | 4 | `nm -D` diff both ways, internal-table privacy, harness sanity |

Both libraries are always loaded with `libloading` and called **only** through
the exported `crc16` symbol — the Rust crate is never linked directly, so the
`#[unsafe(no_mangle)] extern "C"` wrapper is itself under test.

## Mutation testing — proof the suite is not vacuous

Passing tests mean nothing if the harness cannot detect a wrong answer. 12
mutations were injected into `src/lib.rs`, each rebuilt and run through the full
suite:

| mutation | verdict | expected |
|---|---|---|
| M1 block `tables[7]` → `tables[6]` | KILLED (7 tests) | bug |
| M2 block threshold `len >= 8` → `len >= 16` | **SURVIVED** | *equivalent* ✅ |
| M3 drop the `len == 0` early return | KILLED (SIGSEGV in E2) | bug |
| M4 block index `crc >> 8` → `crc & 0xFF` | KILLED (7) | bug |
| M5 tail `wrapping_shl(8)` → `wrapping_shr(8)` | KILLED (8) | bug |
| M6 tail `tables[0]` → `tables[1]` | KILLED (8) | bug |
| M7 block byte order `d[0],d[1]` swapped | KILLED (7) | bug |
| M8 tail index drops the `^ byte` | KILLED (8) | bug |
| M10 block lane `d[5]` → `d[4]` | KILLED (7) | bug |
| M11 tail loop `while len != 0` → `while len > 1` | KILLED (8) | bug |
| M12 seed argument ignored | KILLED (11) | bug |
| M13 `crc & 0xFF` → `crc & 0x7F` | KILLED (7) | bug |

**11 / 11 semantic bugs killed.** M2 survives *correctly*: the slice-by-8 body is
a pure optimisation of the byte-at-a-time tail loop, so raising the threshold
routes bytes through the tail path and yields the identical CRC. This was
confirmed empirically against the C library itself (`block path == chained tail
path`, 500/500 random cases), so M2 is a semantically-equivalent mutation, not a
blind spot.

M3 is notable: it kills via a hard crash rather than a wrong value. The C guards
`d` behind `while (len >= 8)` / `while (len--)`, so `crc16(NULL, 0, seed)` is
well defined in C and returns the seed. The Rust needs the explicit
`if len == 0 { return crc; }` because `slice::from_raw_parts(null, 0)` is UB.
Row E2 is what pins this down.

## Notable C semantics deliberately preserved

* **`len == 0` never dereferences `d`.** `crc16(NULL, 0, seed) == seed` in both
  implementations (E2). The Rust early-return exists solely to keep this
  well-defined.
* **`while (len--)` on `len == 0`** evaluates `0` (false) and exits even though
  `len` wraps to `0xFFFFFFFF`; the loop must not run 4 G times (E7).
* **Integer promotion + truncation.** In C, `crc16 << 8` and `d[0] << 8 | d[1]`
  are computed as `int` and truncated on assignment to `tflac_u16`. Truncation
  distributes over `^`, so the Rust's 16-bit `wrapping_shl(8)` is exactly
  equivalent — verified exhaustively over all 65 536 seeds (C10, E1).
* **`tflac_crc16_tables` stays private.** It is `static const` in the header, so
  it is not an exported symbol; the Rust keeps it module-private
  (`static_table_is_not_exported_by_either_library`).
* **No error surface.** `crc16` has one `return`, no asserts, no null checks, no
  enums — so there is no error code or sentinel to compare. `ERRORS.md` documents
  this derivation and covers the degenerate/boundary inputs instead.
* **Bytes past `len` are ignored** — confirmed by scribbling over them and
  re-checking (E3, C19).

## Data verification

All 8 × 256 = **2048** `tflac_crc16_tables` entries were compared mechanically by
parsing the hex literals out of `c_src/include/lib.h` and `src/tables.rs`:
`C count: 2048  Rust count: 2048  identical: True`.

Reference value cross-check: `crc16("123456789", 9, 0) == 0xFEE8` in both.

## Files changed

* `Cargo.toml` — added `[dev-dependencies] libloading = "0.8"` (only change).
* `tests/common/mod.rs`, `tests/differential_valid.rs`,
  `tests/differential_errors.rs`, `tests/symbol_parity.rs` — new.
* `run_tests.sh`, `SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md`, `VERIFICATION.md` — new.
* `src/lib.rs` and `src/tables.rs` — **unmodified**; no divergence was found, so
  no fix was required. (Restored byte-identical after mutation testing;
  verified with `cmp`.)
* `c_src/` — **unmodified**. The `-O0/-O2/-O3` C builds used out-of-tree build
  directories.
