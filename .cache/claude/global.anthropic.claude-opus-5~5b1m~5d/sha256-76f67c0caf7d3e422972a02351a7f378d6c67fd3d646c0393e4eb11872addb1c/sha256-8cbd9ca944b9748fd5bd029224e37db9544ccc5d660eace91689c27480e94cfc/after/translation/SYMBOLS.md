# SYMBOLS.md — dynamic-symbol parity (Phase A / Phase D)

C shared object: `c_src/build/libharvest-work-TrEEjg.so`
Rust shared object: `translation/target/release/libconvert_pix_lib.so`

Commands used:

```
nm -D --defined-only c_src/build/libharvest-work-TrEEjg.so
nm -D --defined-only translation/target/release/libconvert_pix_lib.so
```

## Full symbol table

| # | symbol | C type | Rust type | in C `.so` | in Rust `.so` | status |
|---|--------|--------|-----------|------------|---------------|--------|
| 1 | `convert_pix`          | `T` (func) | `T` (func) | yes | yes | OK |
| 2 | `cp_inflate`           | `T` (func) | `T` (func) | yes | yes | OK |
| 3 | `cp_error_reason`      | `B` (bss data, `const char *`) | `B` | yes | yes | OK |
| 4 | `cp_fixed_table`       | `D` (data, `uint8_t[320]`) | `D` | yes | yes | OK |
| 5 | `cp_permutation_order` | `D` (data, `uint8_t[19]`)  | `D` | yes | yes | OK |
| 6 | `cp_len_extra_bits`    | `D` (data, `uint8_t[31]`)  | `D` | yes | yes | OK |
| 7 | `cp_len_base`          | `D` (data, `uint32_t[31]`) | `D` | yes | yes | OK |
| 8 | `cp_dist_extra_bits`   | `D` (data, `uint8_t[32]`)  | `D` | yes | yes | OK |
| 9 | `cp_dist_base`         | `D` (data, `uint32_t[32]`) | `D` | yes | yes | OK |

**Missing from Rust: none.** Symbol diff is empty (verified by
`tests/symbols.rs::c_and_rust_export_the_same_symbols`, which shells out to
`nm -D` on both objects and diffs the sorted name sets).

## Not exported (deliberately) — `static` in C, so absent from `nm -D`

These have no dynamic symbol in the C `.so` and therefore must NOT be exported
by Rust either. They *are* translated (present in `src/lib.rs`) and are
exercised transitively through `cp_inflate`:

`cp_make_pixel_a`, `cp_make_pixel`, `cp_would_overflow`, `cp_ptr`,
`cp_peak_bits`, `cp_consume_bits`, `cp_read_bits`, `cp_rev16`, `cp_build`,
`cp_stored`, `cp_fixed`, `cp_decode`, `cp_dynamic`, `cp_block`, `cp_paeth`,
`cp_make32`, `cp_chunk`, `cp_find`, `cp_unfilter`.

`cp_paeth`, `cp_make32`, `cp_chunk`, `cp_find` and `cp_unfilter` are `static`
*and* unreferenced in the C translation unit, so no exported entry point reaches
them. They ARE still tested differentially: `c_src` is built at `-O0`, so the C
`.so` keeps them as LOCAL symbols in `.symtab` (`nm` without `-D` shows
`t cp_paeth`, `t cp_chunk`, `t cp_find`, `t cp_unfilter`, `t cp_make32`), and
`tests/private.rs` recovers their runtime addresses as

    base = dlsym("cp_inflate") - nm_offset("cp_inflate")
    addr = base + nm_offset(name)

The Rust side is reached through `examples/private_probe.rs`, a **test-only**
cdylib that re-exports the same five functions as `probe_cp_*`. That probe is a
separate `.so`; the shipped `libconvert_pix_lib.so` still exports exactly the
nine symbols above (verified by `tests/symbols.rs`, and the five helpers are
merely `pub` in Rust, which does not create a dynamic symbol in a cdylib).

This found and fixed a real bug: in `cp_find` the C writes
`png->p += len + 12;` where the addend is `uint32_t` and therefore
ZERO-extended, while the Rust had `... as c_int as isize`, which
SIGN-extends — so for `len >= 0x7FFFFFF4` the Rust pointer moved ~2 GiB
*backwards* where the C moved forwards. Reintroducing the bug makes
`tests/private.rs::priv_cp_find` segfault while the C is fine, which is the
proof that the test has teeth.

## Undefined (imported) symbols

Rust imports only libc / platform symbols (`memcpy`, `memset`, `abort`,
`malloc`/`free` family via the Rust allocator, `write`, and
`program_invocation_short_name` for the glibc-compatible `assert` message).
0 missing non-libc symbols.

## Cargo features

`translation/Cargo.toml` declares **no `[features]` table** and `src/lib.rs`
contains no `cfg(feature = ...)` gate, so the only buildable configuration is the
default one (`--no-default-features` and `--all-features` are equivalent).
`tests/feature_matrix.rs` asserts both facts, so the claim cannot silently rot.

`./run_all_configs.sh` enumerates the feature combinations mechanically from
`Cargo.toml` and runs the full suite for each, in BOTH cargo profiles (the
`release` profile sets `panic = "abort"` and optimises, so it produces a
genuinely different `.so`). Result: 6 configurations, all passing.

## Test suite

| file | phase | what it does |
|------|-------|--------------|
| `tests/common/mod.rs` | — | dlopens both `.so`s, 4-byte-aligned input buffers with a controllable `addr % 4`, the differential drivers, a fixed-seed xoshiro RNG, and the mutex that serialises access to the shared `cp_error_reason` global |
| `tests/common/deflate.rs` | — | a hand-written raw-DEFLATE encoder (stored / fixed / dynamic blocks, canonical + balanced + skewed Huffman code lengths, code-length RLE, and three streams crafted to trip specific `assert`s) |
| `tests/valid.rs` | B | 38 tests, one per `CONFIGS.md` row |
| `tests/tamper.rs` | B | `CONFIGS.md` rows 35 / 39 (mutating the exported tables) — own process |
| `tests/risky.rs` | B | `CONFIGS.md` row 41 — outcomes range from a normal return to `SIGABRT`, so it is driven in a child process |
| `tests/coverage.rs` | B | re-decodes the generated streams with an independent textbook DEFLATE reader and asserts the claimed block types / RLE symbols are really present |
| `tests/errors.rs` | C | the 14 non-aborting `ERRORS.md` rows |
| `tests/aborts.rs` | C | the 10 `assert`/`SIGABRT` rows, each run in a child process with byte-identical stderr required |
| `tests/private.rs` | B/C | the five `static` helpers no exported symbol reaches, via local-symbol address recovery on the C side and the `private_probe` cdylib on the Rust side |
| `tests/symbols.rs` | D | `nm -D` diff (defined and undefined), plus "private helpers stay private" |
| `tests/feature_matrix.rs` | D | guards the "exactly one feature combination" claim |

81 tests total, all passing in both the `dev` and `release` profiles.

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(... SHARED src/lib.c)` — there
is no driver executable, and the crate has no `[[bin]]`. The "compare binary
stdout" gate is therefore vacuous (documented, nothing to run).
