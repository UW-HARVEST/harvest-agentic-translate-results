# ERRORS.md — Phase A error-surface table

Mechanically derived from every `return` with a non-zero value, every
conditional rejection, and every implicit trap in `c_src/src/lib.c`.
There are no `assert`s, no `RETURN_ERROR` macros, no error enums, no null
checks and no min/max constants in the C source — the entire rejection surface
is the three `return -N` statements plus the implicit traps noted below.

`grep -n 'return\|assert\|NULL' c_src/src/lib.c` yields exactly:

```
78:    const struct caf_audio_description *desc = NULL;
79:    const struct caf_packet_table *pakt = NULL;
80:    const struct ima_block *blocks = NULL;
91:        return -1;
93:        return -2;
122:        return -3;
130:    return 0;
```

## Explicit rejections

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `ima_parse` | `ima_btoh32(header->type) != 'caff'` — first 4 bytes of `data` are not the ASCII bytes `c a f f` (big-endian FourCC `0x63616666`). Covers: wrong magic, all-zero buffer, one-byte-off magic (`caFf`, `cafg`, `Caff`), byte-reversed magic (`ffac`). | returns `-1`, `*info` untouched | `err_row1_bad_magic` |
| 2 | `ima_parse` | magic ok but `ima_btoh16(header->version) != 1` — bytes 4..5 (big-endian u16) are anything other than `0x0001`. Covers `0`, `2`, `0x0100` (LE-swapped 1), `0xFFFF`, and all 65 535 wrong values exhaustively. | returns `-2`, `*info` untouched | `err_row2_bad_version`, `err_row2_bad_version_exhaustive` |
| 3 | `ima_parse` | header ok, chunk scan reached a `data` chunk, but `ima_btoh32(desc->format_id) != 'ima4'` (big-endian FourCC `0x696d6134`). Note the C checks this **after** the loop breaks, so it is reachable only once a `data` chunk was found. Covers: wrong FourCC, `0`, one-byte-off (`ima5`, `IMA4`), reversed (`4ami`). | returns `-3`, `*info` untouched | `err_row3_bad_format_id` |

## Implicit / structural traps (no explicit check in the C — documented, and the
Rust must match where the behaviour is observable)

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| 4 | `ima_parse` | `desc == NULL` at line 118 — a `data` chunk is found before any `desc` chunk. The C dereferences the NULL `desc`. | SIGSEGV (both C and Rust). Not differentially testable in-process; asserted identical by construction (same unguarded load). Verified out-of-process in `err_row4_null_desc_both_crash`. | `err_row4_null_desc_both_crash` |
| 5 | `ima_parse` | `pakt == NULL` at line 125 — `desc` present with valid `ima4` format id, but no `pakt` chunk before the `data` chunk. | SIGSEGV in both. | `err_row5_null_pakt_both_crash` |
| 6 | `ima_parse` | no `data` chunk anywhere: the `for (;;)` loop has **no termination condition** other than finding `data`, so it walks off the buffer forever (or until it faults). | infinite loop / SIGSEGV in both — by construction identical. Exercised out-of-process. | `err_row6_no_data_chunk_hangs` |
| 7 | `ima_parse` | `chunk_size` is negative (`ima_btoh64` of the size field reinterpreted as `ima_s64_t`): `&chunk[1] + chunk_size` moves the cursor **backwards**. Not an error in the C — it is accepted and followed. | no rejection; cursor moves back by `-chunk_size`. Must match. | `cfg_row*_negative_chunk_size` |
| 8 | `ima_parse` | `info == NULL` with an otherwise fully valid buffer: the C writes through it at line 123. | SIGSEGV in both. | `err_row8_null_info_both_crash` |
| 9 | `ima_parse` | `data == NULL`: the C reads `header->type` at line 87. | SIGSEGV in both. | `err_row9_null_data_both_crash` |
| 10 | `ima_parse` | zero-length / truncated `data` buffer (< 8 bytes): the C reads 8 bytes of header unconditionally. With a shorter-but-mapped buffer the read succeeds and the magic/version checks decide. | `-1` or `-2` per rows 1/2, identical in both. | `err_row10_truncated_buffer` |
| 11 | `ima_parse` | `desc->sample_rate` bit pattern out of range for the `double -> unsigned long long` *value* conversion at line 127 (NaN, ±Inf, `>= 2^63`, negative, `< -1`). This is C UB; GCC emits `comisd 2^63 / jae / subsd / cvttsd2si / xor 1<<63`. The Rust must reproduce the same hardware result, including the `0x8000000000000000` "integer indefinite" value. | the exact 64-bit pattern produced by `cvttsd2si`, byte-swapped, reinterpreted as `double` (may be NaN — compared by bits). | `cfg_row*_sample_rate_*`, `err_row11_sample_rate_ub_matrix` |

There are **no** other rejection paths: no range checks, no length checks, no
enum parameters (the public API takes no enum), no min/max constants, and no
`errno` use anywhere in `c_src/`.

## Enum / out-of-range-variant note

`int ima_parse(struct ima_info *info, const void *data)` — the public API has
**no enum parameter**, so the "out-of-range enum value across FFI" class does
not apply. The closest analogue is the FourCC/`version` discriminators, which
*are* covered exhaustively (rows 1–3): every one of the 2^16 `version` values
and a large randomized sweep of 32-bit `type`/`format_id` values are compared
between C and Rust.

---

## Phase C results (all rows checked off)

| # | test | status |
|---|------|--------|
| 1 | `err_row1_bad_magic` (13 hand-picked near-misses + 20 000 randomized types) | ✅ |
| 2 | `err_row2_bad_version` + `err_row2_bad_version_exhaustive` (**all 65 536** values) | ✅ |
| 3 | `err_row3_bad_format_id` (12 near-misses + 20 000 randomized) and `err_row3_bad_format_id_precedes_null_pakt` | ✅ |
| 4 | `err_row4_null_desc_both_crash` (out-of-process, exact signal) | ✅ both SIGSEGV |
| 5 | `err_row5_null_pakt_both_crash` (out-of-process, exact signal) | ✅ both SIGSEGV |
| 6 | `err_row6_no_data_chunk_hangs` (out-of-process, `mmap` + `PROT_NONE` guard page ⇒ deterministic) and `err_row6_long_walk_terminates_identically` (262 000-iteration scan, in-process) | ✅ |
| 7 | `cfg_row08_negative_chunk_size`, `cfg_row22_data_before_desc_via_backjump` | ✅ |
| 8 | `err_row8_null_info_both_crash` | ✅ both SIGSEGV |
| 9 | `err_row9_null_data_both_crash` | ✅ both SIGSEGV |
| 10 | `err_row10_truncated_buffer` | ✅ |
| 11 | `err_row11_sample_rate_ub_matrix` (2 018 values incl. NaN/±Inf/±2^63 boundaries) + `cfg_row11..15` | ✅ |

## Divergences found and fixed during Phase C

1. **`read_unaligned` / plain-deref UB checks (dev profile).** `src/lib.rs` used
   `core::ptr::read_unaligned`, which carries a `debug_assertions`-gated
   `assert_unsafe_precondition!` that panics with `null pointer dereference`
   instead of faulting. With a NULL `data` or NULL `info` the C simply issues the
   load/store and takes SIGSEGV, while the dev-profile Rust `.so` aborted
   (SIGABRT) — a real, observable behavioural difference. Fixed two ways:
   the loads now go through a `#[repr(C, packed)] struct Unaligned<T>` deref (no
   added check, same unaligned machine load), and `[profile.dev]` sets
   `debug-assertions = false` / `overflow-checks = false` so no profile inserts
   checks the C does not have. Verified: `err_row8`/`err_row9` now report SIGSEGV
   from both libraries in both profiles.
2. **Test-harness artefact (not a translation bug).** The first version of the
   row-6 test walked off the end of a plain `Vec`; whether the fault landed in an
   unmapped page (SIGSEGV) or in the thread's stack guard page (which the Rust
   *test harness'* own handler converts to `abort()`) depended purely on ASLR and
   varied run-to-run for the **C** library too. Replaced with an `mmap`'d region
   followed by an explicit `PROT_NONE` guard page, making the fault address —
   and therefore the signal — deterministic for both libraries.
