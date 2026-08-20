# VERIFICATION.md — differential-testing summary

The C in `c_src/` is the ground truth. Every test loads **both** shared objects
with `libloading` and calls the exported `update_frame_header` symbol through
the FFI boundary (the Rust implementation is never called directly, so the
`#[no_mangle] extern "C"` wrapper is itself under test).

```
C    .so : c_src/build/libtranslated_rust.so        (override with $HARVEST_C_SO)
Rust .so : target/<profile>/libupdate_frame_header_lib.so
```

## Artifacts

| file | phase | content |
|------|-------|---------|
| `SYMBOLS.md`  | A / D | every `nm -D` symbol of the C `.so` and its Rust counterpart; feature enumeration |
| `ERRORS.md`   | A / C | error-surface table, 16 rows (E1..E16) |
| `CONFIGS.md`  | A / B | configuration-surface table, 62 rows (R1..R62) |
| `verify_all.sh` | D  | enumerates the feature power set from `Cargo.toml` and runs check/build/symbol-diff/test per combination x profile |

## Test files

| file | rows covered | tests |
|------|--------------|-------|
| `tests/common/mod.rs`          | harness: `dlopen` both `.so`s, 24-byte comparison, fixed-seed xorshift64\* PRNG, class representatives | — |
| `tests/phase_b_configs.rs`     | R1..R61 | 17 |
| `tests/phase_b_deep_sweep.rs`  | R1..R58 re-verified by exhaustive per-axis sweeps | 6 + 5 opt-in |
| `tests/phase_c_errors.rs`      | E1..E16 | 16 + 1 subprocess probe |

## Coverage

| what | cases | result |
|------|-------|--------|
| Phase B, `CONFIGS.md` R1..R62 (randomized per row) | 1 930 000 | 0 mismatches |
| Phase B deep sweeps (contiguous + strided, all-byte compare) | 9 489 000 | 0 mismatches |
| Phase C, `ERRORS.md` E1..E16 | 1 200 000 | 0 mismatches |
| **Exhaustive** `samplerate` = all 2^32 values | 4 294 967 296 | 0 mismatches |
| **Exhaustive** `cur_blocksize` = all 2^32 values | 4 294 967 296 | 0 mismatches |
| **Exhaustive** `bitdepth` = all 2^32 values | 4 294 967 296 | 0 mismatches |
| **Exhaustive** `channels` = all 2^32 values x all 4 channel modes | 17 179 869 184 | 0 mismatches |
| **Exhaustive** `channel_mode` = all 2^8 values | 256 (x 59 392 combos) | 0 mismatches |
| C `.so` built as Debug / Release / RelWithDebInfo / MinSizeRel | full suite x4 | 0 mismatches |
| Rust `.so` built debug / release (`panic = "abort"`) | full suite x2 | 0 mismatches |

### Why the exhaustive per-axis sweeps are equivalent to whole-domain coverage

`update_frame_header` is
`frame_header := 0xFFF80000 | BS(cur_blocksize) | SR(samplerate) | CH(channel_mode, channels) | BD(bitdepth)`
— four branch trees over **disjoint** fields, OR-ed into one word, with no
cross-field dependency other than `CH`, which reads both `channel_mode` and
`channels`. Each axis has been swept over its **complete** domain
(2^32 per u32 axis; 2^8 x 2^32 for the `CH` pair), and the separability itself is
confirmed by the 14 280-combination cross-product (R48), the 2 153 300-case
all-axes-together strided sweep, and 800 000 uniform/structured random full
structs. No `frame_header` value, and no other struct byte, ever differed.

## Divergence found and fixed

One real divergence was found by `ERRORS.md` row **E16** (misaligned `tflac*`):

* **C** dereferences a misaligned `tflac*` and reads/writes the fields anyway
  (x86-64 permits unaligned 32-bit access).
* **Rust, as translated,** built `&mut *t`, which imposes
  `align_of::<tflac>() == 4`; every build with debug assertions trapped with
  `misaligned pointer dereference: address must be a multiple of 0x4` and killed
  the process with `SIGABRT` instead of producing the C's result.

Fixed in `src/lib.rs` by reading and writing the fields **byte-wise through
`*mut u8`** at `offset_of!`-derived offsets, which reproduces the C for every
pointer an FFI caller can pass:

| input | C | Rust before | Rust after |
|-------|---|-------------|------------|
| aligned pointer   | field values | same | same |
| misaligned pointer| performs the access | `SIGABRT` (align check) | performs the access, identical bytes |
| null pointer      | `SIGSEGV` (11) | `SIGSEGV` (11) | `SIGSEGV` (11) |

The byte-wise accessors deliberately avoid `ptr::read_unaligned` /
`ptr::copy_nonoverlapping`, whose debug-mode non-null preconditions would have
turned the null-pointer `SIGSEGV` into a `SIGABRT` and broken row **E1**.
`wrapping_add` is used for offsetting so no `ptr::add` precondition can fire
before the faulting access. No other change to the translation was needed.

## Reproducing

```console
# C shared object
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . && cd ../..

# every feature combination x profile: check, build, nm -D diff, full suite
./verify_all.sh

# the fast suite only (~3 s)
cargo test --no-default-features

# the exhaustive 2^32 sweeps (~10 min, 16 threads)
cargo test --no-default-features --test phase_b_deep_sweep -- --ignored --nocapture --test-threads=1

# against a C object built at another optimisation level
HARVEST_C_SO=/path/to/libtranslated_rust.so cargo test --no-default-features
```
