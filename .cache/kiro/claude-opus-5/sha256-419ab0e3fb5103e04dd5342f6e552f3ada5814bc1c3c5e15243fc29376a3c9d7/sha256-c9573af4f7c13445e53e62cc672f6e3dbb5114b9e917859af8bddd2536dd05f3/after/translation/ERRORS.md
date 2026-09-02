# ERRORS.md — Phase A error-surface table

Derived mechanically from the C source, not from docs or assumptions.

## Mechanical derivation

Grep run over the entire C tree (`c_src/src`, `c_src/include`) for every
rejection / error construct:

```
grep -nE 'return|assert|NULL|ERROR|errno|if *\(|switch|#if|goto|exit|abort|MAX|MIN|<=|>=|[^-]-1' -r src include
→ (no matches)
```

Findings, per construct class:

| construct class searched | occurrences in C |
|--------------------------|------------------|
| error-return macro (`RETURN_ERROR`, …) | 0 |
| `return` of any kind (incl. `return -1`, `return NULL`) | 0 |
| error enum / status code type | 0 |
| `assert` / `abort` / `exit` | 0 |
| `if` / `switch` / `goto` (any conditional rejection) | 0 |
| null-pointer check | 0 |
| explicit range / bounds check | 0 |
| min/max constant | 0 |
| `errno` use | 0 |

`md5_digest` is declared `void`. It has **no return value, no out-parameter
status, no sentinel, and no validation whatsoever** — it is 16 straight-line
unconditional stores. Therefore the C library's error surface is empty: there
exists no input for which the C **reports** an error.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| — | — | *(none: the C contains zero rejection paths)* | — |

**Row count: 0.** There is nothing to check off from this table, because the C
defines no rejection behavior to match.

## Generic-boundary coverage (required by Phase C even when the table is empty)

These are the boundaries every C API has. For each, the C behavior is recorded
below, and the requirement on the Rust is "behave identically", which for the
undefined-behavior cases means "do not add a check the C does not have".

| # | boundary | C behavior | Rust requirement | how covered |
|---|----------|------------|------------------|-------------|
| G1 | `m == NULL` | dereferences NULL → SIGSEGV (UB, no diagnostic) | must also have no null check; must not return early, must not panic with a different signal | `tests/error_paths.rs::null_m_faults_in_both` — runs C and Rust each in a forked child, asserts BOTH die by the SAME signal |
| G2 | `out == NULL` | stores through NULL → SIGSEGV (UB) | same as G1 | `tests/error_paths.rs::null_out_faults_in_both` — same fork/compare-signal method |
| G3 | both pointers NULL | SIGSEGV (UB) | same | `tests/error_paths.rs::null_both_faults_in_both` |
| G4 | `out` buffer shorter than 16 bytes | writes 16 bytes regardless — no length parameter exists, so the C cannot and does not check | Rust must also write exactly 16 bytes and no more/fewer | `tests/error_paths.rs::writes_exactly_16_bytes_no_overrun` — 16-byte window inside a poisoned 48-byte arena; asserts guard bytes on BOTH sides are untouched and identical for C and Rust |
| G5 | "oversized length" | not applicable — the API takes no length argument | n/a | documented; G4 is the analogue |
| G6 | out-of-range enum value across FFI | not applicable — the API has **no enum, no flag, no mode parameter**; the only parameters are two pointers | n/a | documented as vacuous; see below |
| G7 | one step past a documented valid range | not applicable for indices (no index parameter). The value range of every field is the *full* `uint32_t` range, so "one past the range" is unrepresentable; instead the extremes `0x00000000` and `0xFFFFFFFF` and the wrap neighbours `0x00000001` / `0xFFFFFFFE` are tested as values | `tests/error_paths.rs::extreme_word_values` |
| G8 | `out` overlapping / aliasing `m` (caller passes the struct itself as the output buffer) | no restrict qualifier, no aliasing check; C reads each word then stores — result is whatever the store order produces | Rust must produce the identical overlapping result | `tests/error_paths.rs::aliased_out_over_m` |
| G9 | `out` unaligned (odd address) | `tflac_u8*` has alignment 1 — always legal, no check | identical bytes | `tests/error_paths.rs::unaligned_out_offsets` |
| G10 | `m` unaligned (misaligned `tflac_md5*`) | UB per the C standard, but on x86-64 the generated loads are unaligned-tolerant and it reads normally | Rust reads via `read_unaligned`-equivalent semantics must match byte output | `tests/error_paths.rs::unaligned_m_pointer` |

G6 note: because there is genuinely no enum in this API, the "out-of-range enum
variant" bug class cannot be instantiated. The nearest real analogue — an
arbitrary bit pattern arriving from C that has no "valid" interpretation in
Rust — is covered by feeding fully random 16-byte struct images (including
patterns a Rust-side `enum`/`bool`/`NonZero` field would reject) in
`tests/error_paths.rs::arbitrary_struct_bit_patterns`. The Rust struct must
accept all 2^128 images exactly as C does.

## Verification status

`tests/error_paths.rs` covers G1..G10; the table itself has 0 rows, and
`errors_table_is_empty_by_construction` re-derives that emptiness from the C
source at test time, so if the C ever gains a rejection path the test fails and
this table must be regenerated.

The null-pointer rows (G1..G3) are compared by *fault signal*: each library is
called with the null pointer in a forked child process and both must die with
the same signal (SIGSEGV). This is what proves the Rust has not silently added a
null check the C lacks — a Rust version that returned early instead of faulting
would exit 0 and the test would fail.

Result: 10/10 error-path tests pass (plus the 6 `#[ignore]`d death-test payloads
invoked as child processes).

| row | test | status |
|-----|------|--------|
| (table) | `errors_table_is_empty_by_construction` | PASS |
| G1 | `g1_null_m_faults_in_both` | PASS (both SIGSEGV) |
| G2 | `g2_null_out_faults_in_both` | PASS (both SIGSEGV) |
| G3 | `g3_null_both_faults_in_both` | PASS (both SIGSEGV) |
| G4 | `g4_writes_exactly_16_bytes_no_overrun` | PASS |
| G5 | vacuous (no length parameter) | n/a, documented |
| G6 | `g6_arbitrary_struct_bit_patterns` | PASS |
| G7 | `g7_extreme_word_values` | PASS |
| G8 | `g8_aliased_out_over_m` | PASS — **this row found the one real bug**; see `CONFIGS.md` |
| G9 | `g9_unaligned_out_offsets` | PASS |
| G10 | `g10_unaligned_m_pointer` | PASS |
