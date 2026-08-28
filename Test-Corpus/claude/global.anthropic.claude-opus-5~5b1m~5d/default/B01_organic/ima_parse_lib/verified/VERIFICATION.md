# Verification report

C→Rust translation of the single-function CAF/IMA4 parser in `c_src/`.

Everything is exercised **through the FFI boundary only**: both the C `.so` and
the Rust `.so` are `dlopen`ed with `libloading` and called through their exported
`ima_parse` symbol.  The Rust crate is never linked directly, so the
`#[no_mangle] extern "C"` wrapper is itself under test.

## How to run

```bash
cd translation && ./verify.sh          # everything: build C, build Rust, symbols, all tests, both profiles, all feature combos
cd translation && ./check_symbols.sh   # Phase A/D symbol parity only
cd translation && ./mutation_test.sh   # negative control: 31 deliberate mis-translations must all be caught
```

`cargo test` does **not** build a `crate-type = ["cdylib"]` target, so
`cargo build` (or `cargo build --release`) has to run first; `verify.sh` handles
the ordering.

## Artifacts

| file | phase | contents |
|------|-------|----------|
| `SYMBOLS.md` | A / D | `nm -D` inventory for both `.so`s, C-source completeness audit, ABI layout parity table, feature enumeration |
| `ERRORS.md`  | A / C | error-surface table — 10 rejection/fault rows + 6 generic FFI-boundary rows, each with its test |
| `CONFIGS.md` | A / B | configuration-surface table — 17 branch axes, 28 configuration rows, each with its test |
| `tests/support/mod.rs` | — | loader, differential caller, seeded RNG, CAF file builder, guard-page allocator |
| `tests/phase_b_valid.rs` | B | 29 valid-path differential tests |
| `tests/phase_c_errors.rs` | C | 17 error-path differential tests (7 of them via child processes) |
| `verify.sh`, `check_symbols.sh`, `mutation_test.sh` | B/C/D | automation |

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D --defined-only` yields exactly `ima_parse` for
      both libraries; the symbol diff is empty; **0 missing** symbols and **0
      unresolved non-libc** symbols in the Rust `.so`.  The C library is a single
      translation unit and is translated in full (no module was skipped, nothing
      is stubbed).
- [x] **Phase B** — every one of the 28 rows in `CONFIGS.md` passes across
      seeded randomized inputs (~430 000 differential invocation pairs).  Return
      value **and** all 40 bytes of `struct ima_info` (tail padding included,
      `sample_rate` compared bitwise) match byte for byte.
- [x] **Phase C** — every one of the 16 rows in `ERRORS.md` has a passing
      differential test that asserts the *same* error code / sentinel, or (for
      the C library's unchecked NULL derefs, its unbounded chunk scan and its
      self-referential-chunk infinite loop) the *same* termination signal or the
      *same* non-termination.
- [x] **All feature combinations** — `Cargo.toml` declares no `[features]`, so
      the configuration space is `<default>` ≡ `--no-default-features` ≡
      `--all-features`; all three are built and tested, under **both** the `dev`
      and `release` profiles.

```
profile=dev      combo=[<default>]              PASS build / cdylib / symbols / 46 tests
profile=dev      combo=[--no-default-features]  PASS build / cdylib / symbols / 46 tests
profile=dev      combo=[--all-features]         PASS build / cdylib / symbols / 46 tests
profile=release  combo=[<default>]              PASS build / cdylib / symbols / 46 tests
profile=release  combo=[--no-default-features]  PASS build / cdylib / symbols / 46 tests
profile=release  combo=[--all-features]         PASS build / cdylib / symbols / 46 tests
VERIFICATION COMPLETE — all phases passed.
```

## Negative control (proof the suite is not vacuous)

`mutation_test.sh` injects 31 realistic mis-translations into `src/lib.rs`, one
at a time, rebuilds the `.so` and re-runs the suite.  **31 / 31 are caught, in
both profiles.**  Categories:

* struct layout & pointer arithmetic — `sizeof(caf_chunk)` 16→12, `sizeof(caf_header)` 8→12, `sizeof(caf_data)` 4→8, and wrong offsets for `version`, `type`, `format_id`, `channels_per_frame`, `sample_rate`, `frame_count`, and the `blocks` base
* fourcc constants & byte order — `"desc"` fourcc byte-reversed, `ima_btoh32` made the identity, single-bit-wrong masks in `ima_bswap16`/`ima_bswap64`, the `chunk->size` swap removed, the final `sample_rate` swap removed
* control flow & error codes — `-1`→`-2`, `-2`→`-3`, `!= 1`→`!= 0`, "first `desc` wins" instead of "last wins", "first `pakt` wins", `info->size` zeroed, `frame_count` read from `desc`
* the `(ima_u64_t)double` value conversion — saturating `as u64` cast, `to_bits()` bit-cast, `from_bits`→`as f64`, `trunc`→`round`, `<`→`<=` at exactly `2^63`, indefinite value `i64::MIN`→`0`, `xor`→`or` in the bias fixup, bias path removed

Two additional mutants (`x >= 2^63` → `x > 2^63`, and removing the explicit
`is_nan` early-return) survive; both are **provably semantically equivalent**
— at exactly `2^63` the bias path yields `cvttsd2si(0) ^ 2^63 = 0x8000…0` and
the direct path yields the out-of-range indefinite `0x8000…0`, and for NaN the
range comparison is false so the indefinite value is produced either way.  They
are equivalent mutants, not blind spots (and the discriminating neighbour
`t < 2^63` → `t <= 2^63` *is* caught, which pins the boundary).

The `[profile.dev] debug-assertions = false` setting is likewise verified to be
load-bearing: re-enabling it makes `err06`/`err07` fail again.

## Behaviour of the C code that is faithfully reproduced (not "fixed")

1. **`sizeof(struct caf_chunk) == 16`, not 12.** `ima_u32_t type; ima_s64_t size;`
   gets 4 bytes of padding, so the chunk stride is `16 + size` and the on-disk
   chunk header the code reads is *not* the 12-byte header real CAF files use.
   Bytes 4..8 of every chunk are never read.
2. **`ima_btoh*` swap unconditionally**, i.e. the library only decodes
   big-endian correctly on a little-endian host.
3. **Unbounded `for (;;)` chunk scan** — no chunk-count, size or end-of-buffer
   limit; a negative `size` walks the cursor backwards, and `size == -16` makes
   it stand still forever.
4. **`desc` / `pakt` are never NULL-checked.** If the `data` chunk is reached
   first, `desc->format_id` / `pakt->frame_count` dereference NULL.  Note the
   asymmetry the C code creates: the `-3` return happens *before* `pakt` is read,
   so a bad `format_id` with a missing `pakt` returns cleanly (row 24 / `err03_bad_format_id_no_pakt`).
5. **`info` and `data` are never NULL-checked.**
6. **`conv64.u = desc->sample_rate;` is a floating-point → integer *value*
   conversion**, not a bit-cast — the raw big-endian sample-rate bits are
   interpreted as a `double`, truncated toward zero into a `u64` with x86-64
   `cvttsd2si` semantics (`0x8000000000000000` for NaN/inf/out-of-range, and the
   `subsd 2^63` + `xor 2^63` bias sequence for values `>= 2^63`), then *that*
   integer is byte-swapped and reinterpreted as the output `double`.  This is the
   subtlest part of the translation; `double_to_u64` / `cvttsd2si64` in
   `src/lib.rs` reproduce the exact instruction sequence gcc emits (verified
   against `objdump -d` of the C `.so`).
7. **`info->size` is the `data` chunk's `size`**, reinterpreted `s64` → `u64`.
8. **Only the last `desc`/`pakt` before the `data` chunk is used**, and only the
   first `data` chunk (the loop `break`s).
9. **All reads may be misaligned** — the buffer is cast straight to
   `struct caf_*`, so the Rust uses unaligned loads.

## Explicitly out of scope

The *ordering* of the five stores into `*info` on the success path is not
observable through the API (the function returns before the caller can read the
struct), and it is not part of the contract: `gcc -O0` and `rustc -O` schedule
and merge those stores differently.  The only case where it could matter is a
partially-writable `info`, which is UB in C and is already covered at its first
store by `err07_null_info_segv`.
