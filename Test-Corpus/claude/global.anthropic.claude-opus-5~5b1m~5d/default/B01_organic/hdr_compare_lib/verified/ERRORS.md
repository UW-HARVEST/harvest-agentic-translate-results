# ERRORS.md — Phase A: error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. The library contains **no** `RETURN_ERROR`
macro, **no** `assert`, **no** `errno` use, **no** `NULL` check and **no** error enum. Its
entire rejection surface is the set of short-circuiting sub-conditions of two boolean `&&`
chains: every sub-condition that evaluates false makes `hdr_compare` return **exactly `0`**
(a C logical expression yields `int` `0` or `1`).

Grep inventory of every rejection point in the C source:

```
c_src/src/lib.c:4   h[0] == 0xff                          <- rejection 1
c_src/src/lib.c:4   (h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2   <- rejection 2
c_src/src/lib.c:5   (((h[1]) >> 1) & 3) != 0              <- rejection 3
c_src/src/lib.c:5   (((h[2]) >> 4) != 15)                 <- rejection 4
c_src/src/lib.c:6   ((((h[2]) >> 2) & 3) != 3)            <- rejection 5
c_src/src/lib.c:10  hdr_valid(h2)                         <- rejection 6 (+ short-circuit contract)
c_src/src/lib.c:10  ((h1[1] ^ h2[1]) & 0xFE) == 0         <- rejection 7
c_src/src/lib.c:11  ((h1[2] ^ h2[2]) & 0x0C) == 0         <- rejection 8
c_src/src/lib.c:12  !((((h1[2]) & 0xF0) == 0) ^ (((h2[2]) & 0xF0) == 0))  <- rejection 9
```

There is no other `return`, no other statement, and no other branch in the file.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ok |
|---|----------|----------------------------------------------|-------------------|------|----|
| E1 | `hdr_valid` via `hdr_compare` | `h2[0] != 0xFF` (no sync byte). Swept over all 255 non-`0xFF` values, with `h2[1]`/`h2[2]` otherwise perfectly valid and `h1` an exact copy of `h2` | `0` | `e1_h2_byte0_not_ff` | [x] |
| E2 | `hdr_valid` via `hdr_compare` | `h2[1]` matches neither sync class: `(h2[1] & 0xF0) != 0xF0` **and** `(h2[1] & 0xFE) != 0xE2`. Swept over every one of the 238 such byte values | `0` | `e2_h2_byte1_bad_sync_class` | [x] |
| E3 | `hdr_valid` via `hdr_compare` | reserved layer index: `((h2[1] >> 1) & 3) == 0`, i.e. `h2[1] ∈ {0xF0,0xF1,0xE0,0xE1,…}` while byte 0 is `0xFF`. This is the "out-of-range enum value" for the 2-bit layer field | `0` | `e3_h2_layer_reserved` | [x] |
| E4 | `hdr_valid` via `hdr_compare` | bad bitrate index: `(h2[2] >> 4) == 15`, i.e. `h2[2] ∈ 0xF0..=0xFF`. Out-of-range enum value for the 4-bit bitrate field | `0` | `e4_h2_bitrate_index_15` | [x] |
| E5 | `hdr_valid` via `hdr_compare` | reserved sample-rate index: `((h2[2] >> 2) & 3) == 3`, i.e. `h2[2] & 0x0C == 0x0C`. Out-of-range enum value for the 2-bit sample-rate field | `0` | `e5_h2_samplerate_reserved` | [x] |
| E6 | `hdr_compare` | `hdr_valid(h2)` false for **any** reason ⇒ whole comparison rejected *before* `h1` is touched. Exhaustive over all 2^24 values of `h2` (bytes 0..2) against several fixed `h1` | `0` | `e6_invalid_h2_rejects_regardless_of_h1` | [x] |
| E6b | `hdr_compare` | short-circuit contract: `h1` **must not be dereferenced at all** when `hdr_valid(h2)` is false. Driven with `h1 == NULL` and with `h1` pointing at `PROT_NONE` memory, `h2` invalid | `0`, no fault | `e6b_h1_never_dereferenced_when_h2_invalid` | [x] |
| E7 | `hdr_compare` | version/layer mismatch: `((h1[1] ^ h2[1]) & 0xFE) != 0`. Exhaustive over all 256×256 `(h1[1], h2[1])` pairs, valid `h2[2] == h1[2]` | `0` | `e7_byte1_high7_mismatch` | [x] |
| E8 | `hdr_compare` | sample-rate mismatch: `((h1[2] ^ h2[2]) & 0x0C) != 0`. Exhaustive over all 256×256 `(h1[2], h2[2])` pairs, valid matching `h2[1] == h1[1]` | `0` | `e8_byte2_samplerate_mismatch` | [x] |
| E9 | `hdr_compare` | free-format disagreement: exactly one of `h1[2] & 0xF0`, `h2[2] & 0xF0` is zero. Exhaustive over all 256×256 `(h1[2], h2[2])` pairs | `0` | `e9_byte2_freeformat_xor` | [x] |
| E10 | `hdr_compare` | `h2 == NULL` (C reads `h2[0]` unconditionally) | fatal signal (`SIGSEGV`) — the C has no null check | `e10_h2_null_faults_in_both` (forked) | [x] |
| E11 | `hdr_compare` | `h1 == NULL` **with a valid `h2`** (short circuit does not save it: `h1[1]` is read) | fatal signal (`SIGSEGV`) | `e11_h1_null_valid_h2_faults_in_both` (forked) | [x] |
| E12 | `hdr_compare` | both pointers `NULL` | fatal signal (`SIGSEGV`) | `e12_both_null_faults_in_both` (forked) | [x] |
| E13 | `hdr_compare` | undersized `h2` buffer: only 1 readable byte (`0xFF`) then `PROT_NONE`; C must fault reading `h2[1]` | fatal signal (`SIGSEGV`) | `e13_h2_truncated_after_1_byte` (forked) | [x] |
| E14 | `hdr_compare` | undersized `h2` buffer: 2 readable bytes (`0xFF`, valid `h2[1]`) then `PROT_NONE`; C must fault reading `h2[2]` | fatal signal (`SIGSEGV`) | `e14_h2_truncated_after_2_bytes` (forked) | [x] |
| E15 | `hdr_compare` | undersized `h1` buffer: 1 readable byte then `PROT_NONE`, valid `h2`; C must fault reading `h1[1]` | fatal signal (`SIGSEGV`) | `e15_h1_truncated_after_1_byte` (forked) | [x] |
| E16 | `hdr_compare` | undersized `h1` buffer: 2 readable bytes then `PROT_NONE`, `h2` valid and `h1[1]` matching so evaluation reaches `h1[2]`; C must fault | fatal signal (`SIGSEGV`) | `e16_h1_truncated_after_2_bytes` (forked) | [x] |
| E17 | `hdr_compare` | zero-length / empty view: pointer to a 0-byte allocation at the very end of a mapping | fatal signal (`SIGSEGV`) | `e17_h2_zero_readable_bytes` (forked) | [x] |
| E18 | `hdr_compare` | **no over-read**: `h1` and `h2` each exactly 3 readable bytes at the end of a mapping followed by `PROT_NONE`. Neither implementation may read index ≥ 3 | normal return, value equal to the heap-buffer result | `e18_no_read_past_index_2` | [x] |
| E19 | `hdr_compare` | out-of-range "enum" cross-product: every combination of the reserved layer (`0`), bad bitrate (`15`) and reserved sample-rate (`3`) field values — i.e. every invalid bit-field encoding a caller can push across the FFI boundary | `0` | `e19_all_reserved_field_encodings` | [x] |
| E20 | `hdr_compare` | return-value domain: the result must be **exactly** `0` or `1` for every input (a C `&&` chain never yields another truthy int); checked over 2^24 exhaustive `h2` and all randomized runs | `0` or `1`, never anything else | `e20_return_value_is_strictly_0_or_1` | [x] |
| E21 | `hdr_valid` via `hdr_compare` | byte-level short circuit: `h2[0] != 0xFF` with only **1** readable byte in `h2` — the C stops at `h[0]` and must not read `h2[1]`. Paired with a control where `h2[0] == 0xFF` and the read of `h2[1]` *must* fault | `0` (no fault); control: `SIGSEGV` | `e21_byte0_short_circuit_no_read_of_byte1` (forked) | [x] |
| E22 | `hdr_valid` via `hdr_compare` | byte-level short circuit: `h2[1]` fails the sync-class or layer check with only **2** readable bytes — `h2[2]` must not be read. Paired with a control where `h2[1]` is accepted and the read of `h2[2]` *must* fault | `0` (no fault); control: `SIGSEGV` | `e22_byte1_short_circuit_no_read_of_byte2` (forked) | [x] |
| E23 | `hdr_compare` | byte-level short circuit: `((h1[1] ^ h2[1]) & 0xFE) != 0` with only **2** readable bytes in `h1` — `h1[2]` must not be read. Paired with a matching-`h1[1]` control that *must* fault | `0` (no fault); control: `SIGSEGV` | `e23_h1_byte1_mismatch_short_circuits_before_byte2` (forked) | [x] |
| G1 | `hdr_compare` | non-null but wildly invalid pointers (`0x1`, `0x2`, `0x3`, `0x7`, `0xDEADBEEF`, `0xFFFFFFFFFFFFFFF8`) as `h1` and as `h2` | same fatal signal in both; and `0` in both when `h2` is invalid (short circuit) | `generic_boundaries_bad_pointers` | [x] |

## Notes on rows that are *not* in the table

* No `errno`, no out-parameter, no error enum, no allocation ⇒ no allocation-failure or
  `-1`/`NULL` sentinel rows exist.
* `h1[0]` is **never** read by the C. That is a valid-path invariant, so it lives in
  `CONFIGS.md` (row C33), not here.
* "One step past a documented valid range" for this API means the reserved encodings of the
  three packed bit-fields; those are rows E3, E4, E5 and their cross-product E19.
