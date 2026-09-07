# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-AqtBW3.so
nm -D --defined-only translation/target/release/libmatrixsum_lib.so
```

The whole C library is a single translation unit (`c_src/src/lib.c`, 178 lines);
`c_src/include/lib.h` declares only `matrixsum`, but the `.so` exports every
non-`static` definition in the file, so the export surface is 7 functions + 1
data object.

## Defined (exported) symbols

| # | symbol | type | C `.so` | Rust `.so` | notes |
|---|--------|------|---------|------------|-------|
| 1 | `init_array` | `T` (func) | yes | yes | `DynamicArray* init_array(size_t)` |
| 2 | `expand_array` | `T` (func) | yes | yes | `int expand_array(DynamicArray*)` |
| 3 | `add_element` | `T` (func) | yes | yes | `int add_element(DynamicArray*, int)` |
| 4 | `free_array` | `T` (func) | yes | yes | `void free_array(DynamicArray*)` |
| 5 | `process_flags` | `T` (func) | yes | yes | `int process_flags(int)` |
| 6 | `calculate_matrix_checksum` | `T` (func) | yes | yes | `int calculate_matrix_checksum(void)` |
| 7 | `matrixsum` | `T` (func) | yes | yes | `int matrixsum(int,int,int,int)` — the only header-declared symbol |
| 8 | `matrix` | `D` (mutable data) | yes | yes | `int matrix[3][4]`, GLOBAL OBJECT, **48 bytes in both** (verified with `readelf -sW`) |

Symbol diff (C-defined minus Rust-defined): **EMPTY**.
Symbol diff (Rust-defined minus C-defined): **EMPTY** (no extra exports; the
crate is `crate-type = ["cdylib"]` so Rust-internal symbols are not exported).

No symbol required a new export wrapper and no C module was left untranslated:
`lib.c` is the only source file in `CMakeLists.txt`, and every definition in it
has a real (non-stub) Rust counterpart in `translation/src/lib.rs`.

## Undefined symbols

C `.so` imports: `malloc`, `realloc`, `free` (+ the usual weak
`_ITM_*`/`__cxa_finalize`/`__gmon_start__` glibc glue).

Rust `.so` imports: the same three allocator symbols — the translation
deliberately calls libc `malloc`/`realloc`/`free` rather than Rust's allocator,
because `init_array` hands the raw pointer across the ABI and `free_array`
takes it back — plus libc/`libgcc` runtime imports pulled in by the Rust
standard library (`memcpy`, `memset`, `_Unwind_*`, `dl_iterate_phdr`, thread
keys, etc.).

**0 missing / 0 undefined non-libc symbols in the Rust `.so`.**

## Configurations

`translation/Cargo.toml` has **no `[features]` section**, so the only build
configuration is the default one; `--no-default-features` and the empty feature
set are the same build. There is no `[[bin]]` target and no `main.rs`, so there
is no driver executable whose stdout could be compared.

## Completion gate (Phase D)

- [x] `nm -D`: 0 missing symbols in the Rust `.so`, and 0 undefined non-libc
      symbols. The diff is empty in **both** directions. Enforced mechanically
      by `tests/phase_d_symbol_parity.rs`, not just checked by hand.
- [x] Phase B: all 20 `CONFIGS.md` rows pass (`tests/phase_b_valid_paths.rs`,
      20/20) across randomized inputs from fixed SplitMix64 seeds.
- [x] Binary/driver stdout comparison: **N/A** — neither tree builds an
      executable (`CMakeLists.txt` declares only `add_library(... SHARED)`;
      `Cargo.toml` has only `[lib] crate-type = ["cdylib"]`, no `[[bin]]`).
- [x] Phase C: all 15 `ERRORS.md` rows covered
      (`tests/phase_c_error_paths.rs`, 16/16). 13 rows have a passing
      differential test; rows 1 and 12 are fixed-size `malloc` failures that are
      unreachable across the FFI boundary and are documented as such.
- [x] Every configuration: `scripts/verify_all_features.sh` enumerates the
      feature power set and re-runs build + symbol diff + all phases per
      combination. There are no declared features, so it verifies the 2
      configurations that exist (`default`, `--no-default-features`) — both pass.

Additional robustness checks run beyond the required matrix:

- The suite also passes with the Rust `.so` built in the **debug** profile
  (`MATRIXSUM_RUST_SO=target/debug/...`). Because `debug` enables
  `overflow-checks`, this additionally proves the translation uses wrapping
  arithmetic everywhere the C relies on `int` wraparound — an
  `INT_MIN`/`INT_MAX` input would otherwise panic instead of returning a value.
- The suite also passes against the C `.so` built at `-O2`/`-O3`
  (`CMAKE_BUILD_TYPE=Release` and `RelWithDebInfo`, via `MATRIXSUM_C_SO=`), so
  the agreement does not depend on the compiler's treatment of the signed
  overflow in `sum * 0x10`.

Two divergences surfaced during verification; both were defects in the test
harness, not in the translation, and both are documented at the code:

1. `matrix` is an exported *mutable* object and `libtest` runs tests as threads
   in one process, so tests mutating it corrupted concurrent ones. Fixed with a
   process-wide lock held by `Pair` (`tests/common/mod.rs`).
2. Probing multi-gigabyte `init_array` capacities held the C and Rust buffers
   simultaneously, exceeding a ~6 GiB process ceiling and making whichever ran
   second return `NULL` spuriously. Fixed with `probe_init_sequential`, which
   allocates, snapshots, and frees one implementation before the other.
