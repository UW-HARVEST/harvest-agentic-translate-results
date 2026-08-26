# Verification summary

Differential verification of the Rust translation in `src/lib.rs` against the C
ground truth in `c_src/`. Both are built as shared objects and loaded with
`libloading`; the tests **only** ever call the exported `read_side_info` symbol
through `dlsym`, never a Rust function directly, so the `#[no_mangle]`
`extern "C"` wrapper and the struct ABI are part of what is under test.

Reproduce everything with:

```
bash scripts/verify.sh
```

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` on the C `.so` exports exactly one symbol,
      `read_side_info`; the Rust `.so` exports it too. Symbol diff is empty for
      both the dev and the release profile. `nm -D -u` on the Rust object shows
      0 non-libc undefined symbols, and both objects load under `RTLD_NOW`.
      (`tests/phase_d_symbols.rs`, 5 tests)
- [x] **Phase B** — all **38** rows of `CONFIGS.md` pass, each over many
      randomized inputs from a fixed SplitMix64 seed (the unconstrained sweep
      alone runs 20 000 random header/buffer/pos/limit combinations).
      (`tests/phase_b_configs.rs`, 38 tests)
- [x] **Phase C** — all **23** rows of `ERRORS.md` have a passing error-path
      differential test, each asserting the *same* sentinel (`-1`, or the same
      fatal signal for the null-pointer rows), not merely "both failed".
      (`tests/phase_c_errors.rs`, 23 tests)
- [x] **All feature combinations** — the crate declares **no** `[features]` and
      `src/lib.rs` contains no `#[cfg(feature = …)]`, so the power set of
      features is `{∅}`. `scripts/check_all_features.sh` mechanically derives
      the feature list from `Cargo.toml`, enumerates the power set, and runs
      `cargo check` + `cargo build` + `cargo test` for every entry plus the
      `--no-default-features` / default / `--all-features` spellings. All green.
      The suite additionally runs against the **release** `.so` (different
      codegen: `-O3`, `panic = "abort"`), also green.

Totals: 66 tests × 3 configurations, 0 failures.

## What is compared on every call

| observable | how |
|---|---|
| return value (`main_data_begin` or `-1`) | exact `i32` compare |
| `bs->pos`, `bs->limit` after the call | exact `i32` compare (`pos` is advanced even on truncation, so it is a real observable) |
| all 6 `L3_gr_info_t` slots, bytes 8..32 | exact byte compare — the struct has **no** interior padding, and slots past `gr_count` catch out-of-bounds writes |
| `sfbtab` (a pointer into each library's own `.rodata`) | cross-granule pointer *deltas* must match, the 23/40 pointed-to table bytes must match, and slots the C never assigns must still hold the caller's sentinel |

The `L3_gr_info_t` array is pre-filled with a fresh random sentinel per
iteration, which is what makes "the C never writes this field" (e.g.
`region_count[2]` on the window-switching path, or everything after
`block_type` when `block_type == 0`) an *asserted* property rather than an
assumption.

## Harness self-check (mutation testing)

To prove the tests can actually see a divergence, 15 deliberate bugs were
injected into `src/lib.rs` one at a time, rebuilt, and the suite re-run:

| injected bug | detected |
|---|---|
| `big_values > 288` → `>= 288` | yes (36 tests fail) |
| `scalefac_compress` width `4/9` swapped | yes (38) |
| `n_long_sfb` `8/6` swapped | yes (24) |
| `get_bits` limit check `>` → `>=` | yes (3) |
| `scfsi &= 0x0F0F` → `0x0F0E` | yes (17) |
| one byte changed in `g_scf_long[3]` | yes (31) |
| long-table row stride 23 → 22 | yes (32) |
| assign `region_count[2]` on the W=1 path | yes (32) |
| final range check `>` → `>=` | yes (2) |
| `table_select[1]` mask `31` → `63` | yes (37) |
| `sr_idx -= (sr_idx != 0)` → `(sr_idx > 1)` | yes (2) |
| `preflag` threshold `>= 500` → `> 500` | yes (10) |
| `scfsi` field width `7+gr_count` → `8+gr_count` | yes (29) |
| `main_data_begin >> gr_count` → `>> 1` | yes (21) |
| padding bytes past `&g_scf_mixed[8]` `0` → `1` | **no** (see below) |

14 of 15 mutations are caught. The one that is not is the only region where the
C's behaviour is not reproducible in principle — see below.

## The one documented, irreproducible divergence

`sr_idx` can reach 8 (`hdr[1]` bits 3 and 4 set, `hdr[2]` bits 2-3 = 3), which
indexes one row past the end of every `[8][…]` scalefactor-band table. The C has
no bounds check, so it reads out of bounds:

| C expression | what it reads | Rust |
|---|---|---|
| `&g_scf_long[8]` | 8 zero pad bytes + `g_scf_short[0][0..15]` | **identical** (asserted, `err_16`) |
| `&g_scf_short[8]` | `g_scf_mixed[0]` | **identical** (asserted, `err_16`) |
| `&g_scf_mixed[8]` | past the end of `.rodata`, i.e. `.eh_frame_hdr` — link-time unwind data full of PC-relative offsets | 40 deterministic zero bytes |

The last row cannot be matched by any translation: those bytes differ between
two builds of the *same* C source (and gcc even reverses the table order at
`-O1`+, which changes all three aliases). `src/lib.rs` reproduces the reference
`-O0` layout exactly (`long` +0, 8 pad bytes, `short` +192, `mixed` +512) so the
first two rows match byte-for-byte, and supplies zeros beyond +832 so the Rust
can never fault. For that one case the tests still assert that the return value,
all 24 non-pointer struct bytes of every granule, `bs->pos` and the *pointer
offset* (`&g_scf_mixed[8]` is 320 bytes past `&g_scf_short[8]` in both
libraries) are identical — only the byte comparison of that one out-of-bounds
row is skipped, and `ERRORS.md` row 16 documents why.

## `[profile.dev]` note

`Cargo.toml` sets `debug-assertions = false` / `overflow-checks = false` for the
dev profile. Reason: rustc's `-C debug-assertions` also enables `-Z ub-checks`,
which injects a `"null pointer dereference occurred"` panic in front of every
raw-pointer dereference. gcc inserts no such check (the reference build is plain
`-O0`, no sanitizers), so with UB-checks on, a null `bs_t *` made the Rust abort
(SIGABRT) where the C faults (SIGSEGV) — a difference introduced by the Rust
toolchain's debugging aids, not by the translation. With the dev profile
configured like the C build (and like the release profile that consumers link),
the two libraries terminate identically on all four null-pointer inputs
(`err_17`…`err_20`, verified by comparing child-process exit signals).

## Changes made to the crate

Only these, all outside `src/lib.rs` (whose behaviour needed no fix — the
translation was already byte-exact on every input the tests could construct):

| file | change | why |
|---|---|---|
| `Cargo.toml` | `[dev-dependencies] libloading = "0.8"` | load both `.so`s through `dlopen`/`dlsym` |
| `Cargo.toml` | `[lib] crate-type = ["cdylib", "rlib"]` | with `cdylib` alone, `cargo test` does **not** emit the `.so`; adding `rlib` makes cargo build (and re-build) the cdylib as part of `cargo test`, so the object under test can never be stale or missing. Exported symbol set is unchanged (`nm -D` ⇒ `read_side_info` only). |
| `Cargo.toml` | `[profile.dev] debug-assertions = false`, `overflow-checks = false` | see the `[profile.dev]` note above |
| `tests/common/mod.rs` | shared harness (lib loading, bit writer, comparators, RNG) | — |
| `tests/phase_b_configs.rs` | 38 tests, one per `CONFIGS.md` row | — |
| `tests/phase_c_errors.rs` | 23 tests, one per `ERRORS.md` row | — |
| `tests/phase_d_symbols.rs` | 5 symbol-parity tests | — |
| `scripts/check_all_features.sh`, `scripts/verify.sh` | automation | — |
| `SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md`, `VERIFICATION.md` | Phase A artifacts + this summary | — |

Nothing in `c_src/` was modified; the only addition there is the `build/`
directory produced by the documented `cmake` command (the harness creates it
automatically if it is missing, so a bare `cargo test` works on a clean tree).
