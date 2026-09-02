# Verification report

The Rust crate in this directory is a translation of `../c_src/src/lib.c`
(stb_ds by Sean Barrett, inlined, plus the `strkey` / `arr_ins` helpers).
The C code is the ground truth.

## How to reproduce

```sh
# 1. build the C shared library
cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. build the Rust cdylib and run everything, under every feature combination
cd ../../translation
./check_all_features.sh
```

`check_symbols.sh` alone re-checks `nm -D` parity.

Every test loads BOTH `.so`s with `libloading` and calls only exported
symbols — the Rust side is always reached through its `#[no_mangle] extern "C"`
wrappers, never linked directly — so the export wrappers are under test too.
The suite must be run with `--test-threads=1`: `dlopen` returns one handle per
`.so` per process, so all tests share the libraries' process-global state
(`stbds_hash_seed`, `strkey`'s static buffer).

## Artifacts

| file | contents |
|------|----------|
| `SYMBOLS.md` | all 16 C dynamic symbols and their Rust counterparts; 0 missing |
| `CONFIGS.md` | 45 valid-input configuration rows (Phase B gate), all ticked |
| `ERRORS.md` | 44 distinct rejection/error behaviours (Phase C gate) |
| `tests/common/mod.rs` | loader, fixed-seed SplitMix64 PRNG, stb_ds macro emulation, snapshot + trace comparison |
| `tests/phase_b.rs` | 45 valid-path differential tests |
| `tests/phase_c.rs` | 43 error-path differential tests (5 via subprocess signal comparison) |
| `tests/phase_d.rs` | symbol parity, ABI/layout parity, completion gate |

## What is compared

Addresses are never comparable across two independently loaded libraries, so
every assertion is over values only. After each operation the traces record:

* every element byte of the array (`length * elemsize`); the drivers write the
  full element so no padding or uninitialised `realloc` bytes are compared;
* for string maps, where the key slot holds a pointer, the key's string
  **content** plus the value bytes;
* the array header — `length`, `capacity`, `temp`, `hash_table != NULL`;
* the entire `stbds_hash_index`: `slot_count`, `used_count`,
  `used_count_threshold`, `used_count_shrink_threshold`, `tombstone_count`,
  `tombstone_count_threshold`, `seed`, `slot_count_log2`, `string.remaining`,
  `string.block`, `string.mode`, and every `hash[]` / `index[]` slot of every
  bucket;
* return values: indices, `temp`, `temp_key` contents, hashes, C strings;
* for crash rows, the child process's `(exit code, signal)`.

`stbds_rand_seed` is called on both libraries before each scenario, because
`stbds_make_hash_index` advances a process-global seed LCG, making the seed part
of the observable state.

## Results

```
phase_b : 45 passed, 0 failed
phase_c : 43 passed, 0 failed   (+1 ignored: the subprocess crash_runner driver)
phase_d :  8 passed, 0 failed
symbol diff: empty
```

Green under the default features, under `--no-default-features` (the crate
declares no optional features, so those are the only two configurations), and
against both the release cdylib and the `debug_assertions`-enabled debug cdylib
(`DIFF_RUST_PROFILE=debug`) — which also shows that no code path relies on Rust
arithmetic that would panic where the C wraps.

## Changes made to the Rust during verification

1. `src/lib.rs`, `stbds_stralloc`: `512 << (a->block >> 1)` now uses
   `wrapping_shl`. `a->block` is a public `unsigned char` field, so the shift
   count can reach 127; gcc emits `shlq %cl`, which masks the count to 6 bits on
   x86-64. The previous plain `<<` is a Rust arithmetic-overflow (LLVM poison)
   for counts >= 64. It happened to codegen identically in this build, but
   `wrapping_shl` makes the x86 masking semantics explicit instead of
   accidental. Exercised for all 256 `block` values by
   `cfg_38_stralloc_block_field`.

No other divergence was found. Everything else in this report is test
infrastructure; `c_src/` was not modified.

## Faithfully-reproduced C quirks that were explicitly confirmed

* siphash's `int`-width block load sign-extends into `size_t` when byte 3 or 7
  has its high bit set, while the `case 7/6/5` tail bytes are cast to `size_t`
  *before* shifting and therefore zero-extend (`cfg_02`).
* `stbds_hash_string`'s `(unsigned char)` cast zero-extends bytes >= 0x80
  (`cfg_05`, `err_24`).
* `mode` is dispatched with `mode >= STBDS_HM_STRING`, so any `int` >= 1 is a
  "string" mode and any `int` < 1 is "binary" — including out-of-range values
  like `INT_MIN`, `-1`, `2`, `7`, `INT_MAX` (`cfg_34`, `err_31`).
* `stbds_hmdel_key` gates the strdup `free` on `mode == STBDS_HM_STRING`
  *exactly*, so `mode == 2` hashes as a string but takes the **binary** re-find
  path (`cfg_33`, `err_33`, `err_33b`).
* in `stbds_hmput_key`'s found-key handling, the first inner probe loop sets
  both `stbds_temp` and `stbds_temp_key`, and the wrap-around loop sets only
  `stbds_temp` (`cfg_45` records `temp_key` after every put).
* `stbds_shmode_func` truncates `mode` with `(unsigned char)`, so 256 → 0,
  −1 → 255, 259 → ARENA (`cfg_35`, `err_34`).
* `string.mode` outside {1,2,3} hits the `switch` `default:` and `memcpy`s the
  first `keysize` bytes **of the string** into the key slot instead of a pointer;
  a later lookup then dereferences those bytes as a `char*` (`cfg_29`, `cfg_30`,
  `err_35`, `err_35b`).
* `stbds_hmput_key` / `stbds_hmget_key_ts` hard-code `keyoffset = 0`; only
  `stbds_hmdel_key` takes one, so a non-zero `keyoffset` makes deletes miss
  (`cfg_22`, `err_37`).
* `stbds_arrgrowf(NULL, es, 0, 0)` returns `NULL` without allocating, because
  `min_cap <= stbds_arrcap(NULL)` is `0 <= 0` (`err_17`).
* `keysize == 0` makes every key hash and compare equal, so the first key
  permanently shadows all others (`err_36`).
