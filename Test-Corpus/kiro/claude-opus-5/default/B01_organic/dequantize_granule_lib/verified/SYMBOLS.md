# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D` on both shared objects.

C library:   `c_src/build/libharvest-work-mJBc45.so`
Rust library: `translation/target/release/libdequantize_granule_lib.so`

## C source inventory (completeness check)

The whole C library is two files:

| C file | functions defined | linkage | exported? |
|--------|-------------------|---------|-----------|
| `c_src/src/lib.c` | `get_bits` | `static` | no (internal) |
| `c_src/src/lib.c` | `dequantize_granule` | external | yes |
| `c_src/include/lib.h` | (types only: `bs_t`, `L12_scale_info`) | — | n/a |

`c_src/CMakeLists.txt` lists exactly one translation unit (`src/lib.c`) and
builds a single SHARED target. There is no second module, no binary/driver
target, and no `#ifdef`-gated extra source. So the complete external surface is
one function. Nothing in the C tree is untranslated.

## `nm -D --defined-only`, non-`_`-prefixed symbols

| # | symbol | in C `.so` | in Rust `.so` | action |
|---|--------|-----------|---------------|--------|
| 1 | `dequantize_granule` | `T` (0x11d1) | `T` | none — already exported via `#[unsafe(no_mangle)] extern "C"` |

Symbols only in the Rust `.so` (extra, allowed — toolchain/runtime artifacts,
not part of the C surface): `rust_eh_personality`, `_init`, `_fini`,
`__bss_start`, `_edata`, `_end` and the `_ITM_*` / `__gmon_start__` weak refs.

## Symbol diff

```
$ comm -23 <(nm -D --defined-only C.so   | awk '{print $3}' | grep -v '^_' | sort) \
           <(nm -D --defined-only RUST.so | awk '{print $3}' | grep -v '^_' | sort)
(empty)
```

**Missing from Rust: 0.**  **Stubs / `unimplemented!()` in Rust: 0.**

## Undefined symbols in the Rust `.so`

All undefined symbols resolve to libc / libgcc_s (glibc `malloc`, `memcpy`,
`__errno_location`, `pthread_*`, `_Unwind_*`, `dl_iterate_phdr`, …) pulled in by
the Rust standard library. There are no undefined *non-libc* symbols, i.e. no
references to un-translated C code.

## ABI types (must match, verified against the C compiler)

Verified with a `gcc` `offsetof`/`sizeof` probe against `c_src/include/lib.h`:

| type | C | Rust `#[repr(C)]` |
|------|---|-------------------|
| `bs_t` | size 16, `buf`@0, `pos`@8, `limit`@12 | identical (`const _:` asserts in `src/lib.rs`) |
| `L12_scale_info` | size 900, align 4, `scf`@0, `total_bands`@768, `stereo_bands`@769, `bitalloc`@770, `scfcod`@834 | identical (`const _:` asserts in `src/lib.rs`) |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
build configuration is the default one. Phase D's "repeat for every feature
combo" reduces to: default, `--no-default-features`. Both are enumerated and
run by `check_features.sh`.

## Verification results (re-run after a clean `rm -rf target`)

```
$ diff <(nm -D --defined-only C.so    | awk '{print $3}' | grep -v '^_' | sort) \
       <(nm -D --defined-only RUST.so | awk '{print $3}' | grep -v '^_' | sort)
(no differences)

$ nm -D --defined-only target/release/libdequantize_granule_lib.so | grep dequantize_granule
00000000000116d0 T dequantize_granule
```

`check_features.sh` re-verifies this for every configuration × profile:

```
features declared in Cargo.toml: 0 (none)
=== configuration: default  (cargo ) ===
  symbol parity [default/release]: OK (1 C symbol(s), 0 missing, 0 undefined non-libc)
  differential suite [default/release]: PASS
  symbol parity [default/debug]: OK (1 C symbol(s), 0 missing, 0 undefined non-libc)
  differential suite [default/debug]: PASS
=== configuration: no-default  (cargo --no-default-features) ===
  symbol parity [no-default/release]: OK (1 C symbol(s), 0 missing, 0 undefined non-libc)
  differential suite [no-default/release]: PASS
  symbol parity [no-default/debug]: OK (1 C symbol(s), 0 missing, 0 undefined non-libc)
  differential suite [no-default/debug]: PASS

PHASE D: ALL CONFIGURATIONS PASS
```

## Fix applied during verification

One real divergence was found, and only by building the Rust `.so` with the
`debug` profile:

* **`Cargo.toml`** — Rust's `debug_assertions`-gated UB checks fired on the
  deliberate null / out-of-bounds accesses this translation must reproduce,
  turning the C's `SIGSEGV` into a Rust panic and `SIGABRT` (test
  `e26_null_pointers`: `C: KILLED by signal 11` vs `R: KILLED by signal 6`).
  `debug-assertions = false` / `overflow-checks = false` are now set in **both**
  profiles so the library's observable behaviour does not depend on how it is
  built. No change to `src/lib.rs` was required.

## Suite trustworthiness (mutation testing)

A passing differential suite only means something if it *can* fail.
`mutation_check.sh` builds eight deliberately-wrong copies of `src/lib.rs` into
their own `.so`, points the suite at each via `RUST_SO`, and requires the suite
to fail. All eight are caught:

```
MUTANT guard-ge:        DETECTED   (exhaustion guard `>` changed to `>=`)
MUTANT choff-19:        DETECTED   (`choff = 18 - choff` changed to 19)
MUTANT shl-not-masked:  DETECTED   (out-of-range shift zeroed instead of masked)
MUTANT sat-sub:         DETECTED   (`wrapping_sub` changed to `saturating_sub`)
MUTANT ba-masked-idx:   DETECTED   (`bitalloc[i]` index masked to 6 bits)
MUTANT half-off-one:    DETECTED   (`half` off by one)
MUTANT pos-not-advanced:DETECTED   (`bs->pos` not advanced on the reject path)
MUTANT mod-shift-wrap:  DETECTED   (`2 << (ba-17)` shift count clamped)
```
