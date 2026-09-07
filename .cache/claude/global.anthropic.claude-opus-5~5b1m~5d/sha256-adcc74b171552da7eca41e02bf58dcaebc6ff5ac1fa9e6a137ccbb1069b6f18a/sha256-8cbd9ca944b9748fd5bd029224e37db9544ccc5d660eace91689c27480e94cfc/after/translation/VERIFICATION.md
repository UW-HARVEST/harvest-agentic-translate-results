# Verification report

Differential verification of the Rust translation in `src/lib.rs` against the C
ground truth in `../c_src/src/lib.c`.

Reproduce everything with:

```sh
cd translation && ./run_verification.sh
```

Both libraries are loaded with `libloading` and called **only** through their
exported symbols, so the `#[no_mangle] extern "C"` wrappers are themselves under
test — no Rust function is ever called directly.

## What is built

| artefact | how | notes |
|----------|-----|-------|
| C, asserts **live** | `cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .` | the task's own command; `CMAKE_BUILD_TYPE` is empty so `NDEBUG` is *not* defined and `assert()` calls `__assert_fail` |
| C, `NDEBUG` | `cmake -S c_src -B cbuild_release -DCMAKE_BUILD_TYPE=Release -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build cbuild_release` | the configuration the translation targets |
| Rust cdylib | `cargo build --release` | `target/release/libunfilter_lib.so` |

Both C builds are tested against the Rust `.so`.

## Test layout

| file | phase | contents |
|------|-------|----------|
| `tests/common/mod.rs` | harness | `dlopen`s both `.so`s; `diff_unfilter*` (in-process) and `diff_inflate_batch` (**fork-isolated**, shared-memory arena) |
| `tests/common/deflate.rs` | harness | a small DEFLATE *writer* (bit writer, canonical Huffman, fixed/stored/dynamic block emitters, complete-tree constructors) so tests can aim at individual `cp_inflate` code paths |
| `tests/symbols.rs` | D | `nm -D` parity, unresolved-import check, byte-identical exported data tables |
| `tests/phase_b_unfilter.rs` | B | `CONFIGS.md` rows 1–15 |
| `tests/phase_b_inflate.rs` | B | `CONFIGS.md` rows 16–34 |
| `tests/phase_c_errors.rs` | C | `ERRORS.md` rows 1–34 + null pointers + a 2000-case malformed-input fuzz |

`cp_inflate` is run in a forked child because the assert-enabled C build dies
with `SIGABRT` on malformed input, and because in `Release` a few corrupt
dynamic-Huffman headers make the C code write outside its own stack frame.
Isolating each call lets the harness tell "returned an error" apart from "the C
library self-destructed", and keeps one bad input from taking the run down.  A
child processes as many cases as it survives; when it dies the parent marks that
case and re-forks after it.

## Completion gate

- [x] **`SYMBOLS.md`: symbol diff EMPTY.**  Both `.so`s export exactly the same
      9 symbols (`cp_dist_base`, `cp_dist_extra_bits`, `cp_error_reason`,
      `cp_fixed_table`, `cp_inflate`, `cp_len_base`, `cp_len_extra_bits`,
      `cp_permutation_order`, `unfilter`), with matching `nm` kinds.  Nothing
      was stubbed: `src/lib.rs` translates every function in `lib.c`, including
      the `static` ones that are dead in the C translation unit too
      (`cp_make_pixel*`, `cp_make32`, `cp_chunk`, `cp_find`).  The Rust `.so`
      has 0 unresolved non-libc imports.  Asserted by
      `tests/symbols.rs::rust_so_exports_every_c_symbol` and
      `::rust_so_has_no_unresolved_non_libc_symbols`.
- [x] **Phase B: every row of `CONFIGS.md` (1–34) passes** across randomized
      inputs from the fixed seed `0x5EED_1234`, comparing the return value, the
      **whole** output buffer, and `cp_error_reason`.  Happy-path rows also
      assert the decode actually succeeded and produced the expected plaintext,
      so "both fail identically" cannot masquerade as a pass.  Coverage
      includes: all 5 filter types × row-0/row-`y` variants × `bpp 1..4`, all 4
      block types, all 29 length symbols and all 30 distance symbols (with the
      extreme value of every extra-bit class), all 16
      `(first_bytes, last_bytes)` alignment combinations, hand-built dynamic
      headers at both extremes (`HLIT 257/288`, `HDIST 1/32`, `HCLEN 5/19`,
      code-length symbols 16/17/18), Huffman codes of depth 1..15, multi-block
      `BFINAL` chains, 64 KiB payloads, 32 KiB back-references, and the composed
      `cp_inflate` → `unfilter` pipeline.
- [x] **No binary target exists**, so the stdout comparison does not apply:
      `Cargo.toml` declares only `crate-type = ["cdylib"]` (no `[[bin]]`, no
      `src/main.rs`), and `c_src/CMakeLists.txt` declares only
      `add_library(... SHARED)`.  Verified by inspection and by
      `tests/symbols.rs`.
- [x] **Phase C: every row of `ERRORS.md` (1–34) has a passing error-path
      differential test** that asserts the *same* sentinel and the *same*
      `cp_error_reason` string, not merely "both failed".  Plus the generic
      boundaries: `NULL` in/out pointers, `in_bytes`/`out_bytes` of `0`, `-1`
      and `i32::MIN`, and the **full 0..255 domain of the filter byte**
      (the out-of-range-`enum` class, exhaustively, on 1/2/3-row images).
      Rows 13–22 are the `assert()`s: `err13_to_22_asserts_are_live_in_the_assert_build`
      proves they really do fire (`SIGABRT`) in the configuration the task's
      `cmake ..` produces, and `err13_to_22_release_build_matches_rust` proves
      the `NDEBUG` build and Rust agree exactly on the same inputs.
- [x] **All of the above hold under every feature combination.**  `Cargo.toml`
      declares **no `[features]` table**, so the configuration space is a single
      point; `run_verification.sh` enumerates it programmatically anyway and runs
      the full suite under `default`, `--no-default-features` and
      `--all-features`.  All three pass, and the script would expand to the full
      powerset if features were ever added.

## Divergences found

**Zero.**  Across all of Phase B, all of Phase C, and a 2000-case malformed
input fuzz (random bytes, single-bit flips of valid streams, multi-bit flips,
truncations, all four input alignments):

```
fuzz: 2000 cases, 1999 matched, 1 the C library killed itself on, 0 divergences
```

The single unmatched case is one where the C library destroys itself — a corrupt
dynamic-Huffman header yields code lengths `>= 16`, and `cp_build` then indexes
its 16-entry `counts`/`codes`/`first` stack arrays out of bounds (`lib.c:143`,
`lib.c:154`).  That is undefined behaviour whose effect depends on the C
compiler's stack layout, so it is not reproducible by construction; the Rust
translation uses 256-entry tables and stays memory-safe instead.  The
assert-enabled C build catches this input with
`assert(len < 16)` before it can corrupt anything.

Four test *expectations* of mine were wrong and had to be corrected against the
C, never the other way round; all four are genuine C quirks now documented in
`ERRORS.md` §E:

1. `cp_stored` performs **no** output-bounds check (it `memcpy`s `LEN` bytes
   regardless of `out_bytes`).
2. `cp_stored`'s copy source is only correct when the input length is a multiple
   of 4, because `cp_ptr()` does not account for the `final_word` refill path.
3. A stored block can only be the last block in a stream (so miniz level-0
   output over 64 KiB is rejected).
4. With `len == 0` and `bpp > 0`, `unfilter`'s row-`y` prologue overwrites the
   filter bytes of later rows, making its return value data-dependent.

## Known configuration caveat

The verified artefact is the crate's declared `[profile.release]` cdylib
(`panic = "abort"`, `overflow-checks = false`, `debug-assertions = false`).  A
cdylib built with the `dev` profile behaves identically on every input above
*except* the already-UB `NULL`-pointer ones, where `core::ptr`'s own debug
assertion fires and the process dies with `SIGABRT` instead of the C library's
`SIGSEGV`.  This is noted in `tests/phase_c_errors.rs::err_null_pointers`.
