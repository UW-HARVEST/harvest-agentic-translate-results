# VERIFICATION.md — result of the A→B→C→D differential verification

**Outcome: one real divergence found and fixed. After the fix the Rust
translation is behaviourally identical to the C for every input exercised.**

The divergence was on the `item == NULL` path (`ERRORS.md` row M1): under
`-C debug-assertions` — which Cargo's `dev` profile enables by default — the
Rust aborted with `SIGABRT` and a `null pointer dereference occurred` panic
message, where the C raises a silent `SIGSEGV`. Fixed in `src/lib.rs` by routing
the stores to `*item` through `addr_of_mut!` + `ptr::write` (the `item_store!`
macro), which emits the same plain faulting store as the C in every profile.
Details in `ERRORS.md`.

## How to reproduce

```sh
cd translation && ./verify.sh      # symbols + full suite, every combo x profile
cd translation && ./mutate.sh      # proves the suite is sensitive, not vacuous
```

Both scripts are self-contained (they build the C `.so` themselves) and use
`--offline` because the crates.io index is unreachable from this sandbox;
`libloading 0.8.9` and `cfg-if` come from the local registry cache.

## What was tested, and how

Both implementations are reached **only** through `dlopen`/`dlsym` on their
shared objects — never by linking the Rust crate — so the
`#[unsafe(no_mangle)] extern "C"` export wrapper is part of what is under test:

* C: `c_src/build/libdriver.so`
* Rust: `translation/target/<profile>/libdriver.so`

Each case is run against both `.so`s with independent copies of the input, and
the following are compared **byte-for-byte**:

| compared | why |
|----------|-----|
| the `cJSON_bool` return value | the only error channel |
| `item->type` | must become exactly `cJSON_Number` on success, be preserved on failure |
| `item->valueint` | the saturation cascade |
| `item->valuedouble` **as raw `u64` bits** | so `-0.0`, `+inf`, `-inf` and NaN payloads cannot compare equal by accident |
| `parse_buffer.content` / `.length` / `.offset` / `.depth` | `.offset` is the only field the C writes; the other three must be untouched |
| the input bytes themselves | neither implementation may write through `content` |

Every backing allocation is padded with 128 bytes of `0xAB` past the logical
content, so that an over-read is *deterministic* rather than uninitialised heap —
otherwise an over-read would show up as a random, unreproducible "divergence".

## Phase results

### Phase A — surface mapped

| artifact | content |
|----------|---------|
| `SYMBOLS.md` | 1 exported symbol (`parse_number`); C↔Rust symbol diff empty; both `.so`s import the same four libc functions (`malloc`, `free`, `memcpy`, `strtod`) |
| `ERRORS.md` | 15 rows: 4 distinct error returns (E1–E4) with 6 enumerated sub-triggers, 4 boundary rows (B1–B4), and the missing-`item`-null-check row (M1) that turned out to be differentially testable after all |
| `CONFIGS.md` | 35 rows over 12 axes derived from the C's own branch structure, plus 6 cross-cutting matrices |

The C build has exactly one translation unit (`src/lib.c`), and it is fully
translated — no module was skipped, so no symbol was missing for lack of an
implementation and nothing had to be newly translated.

### Phase B — every `CONFIGS.md` row passes

35/35 rows, all randomized rows driven by a fixed-seed xorshift64\* PRNG.
Includes the composed pipeline (`c33`), which streams several numbers out of one
buffer by feeding the advanced `offset` back in, rather than testing one call in
isolation.

### Phase C — every `ERRORS.md` row passes

15/15 rows, each asserting the **exact** sentinel (`0` / `1`) plus the exact side
effects, not merely "both failed". Row M1 (`item == NULL`) asserts the exact fatal
**signal** instead, by re-execing the test binary once per implementation — this
is the row that exposed the one real divergence. Plus the generic FFI boundaries: all NULL
combinations, zero/oversized `length`, every `offset` in `0..=len+1`, one step
past every binary64 range boundary, and out-of-range enum ints in `cJSON.type`
(`INT_MIN`, `INT_MAX`, negatives, every single-bit and inverted-single-bit
pattern) crossed with both success and failure inputs.

Notable exhaustive coverage:

* all 256 possible leading bytes,
* all 3 615 strings of length 1–3 over the 15-byte accepted alphabet,
* all 50 625 strings of length 4, plus 60 000 sampled of length 5–8,
* the full `shape × offset × length` cross-product to length 3 (≈ 57 000 calls),
* an exhaustive 5- and 6-wide sweep over one representative byte per switch arm.

### Phase D — symbol parity and every combination

`Cargo.toml` declares no `[features]`, so the combination sweep is
`{default, --no-default-features, --all-features} × {debug, release}` — 6
configurations. `verify.sh` runs all 6 and for each one re-checks:

* `comm -23` of the two `nm -D --defined-only` symbol lists → **empty**,
* `ldd -r` on the Rust `.so` → **0 undefined symbols**,
* the full 65-test suite → **pass**.

The debug/release split matters even without features, because it changes how
Rust lowers integer overflow checks and float→int casts.

Each of the 6 configurations additionally runs the heavy exhaustive sweeps
(`tests/heavy_exhaustive.rs`, `#[ignore]`d in a plain `cargo test`):

| sweep | cases | result |
|-------|-------|--------|
| all 2¹⁶ byte pairs × 4 lengths × 3 offsets | 786 432 | 0 divergences |
| all 15⁵ strings of length 5 over the accepted alphabet | 759 375 (612 000 accepted / 147 375 rejected) | 0 divergences |
| all 7⁷ strings of length 7 over one byte per switch arm, rotating offset | 823 543 | 0 divergences |
| randomized, all axes varied | 2 000 000 (1 093 861 accepted) | 0 divergences |

≈ **4.37 million differential calls per configuration, ≈ 26 million total, zero
divergences.** The accepted/rejected split is asserted to be non-degenerate, so
the sweeps provably exercise both the success and the failure paths.

## ABI layout, independently confirmed

A standalone C probe (compiled against the untouched `c_src/include/lib.h`) and a
standalone Rust program printing the same facts agree exactly:

```
cJSON        size=16 align=8  type=0 valueint=4 valuedouble=8
parse_buffer size=32 align=8  content=0 length=8 offset=16 depth=24
cJSON_bool   size=4   INT_MAX=2147483647  INT_MIN=-2147483648  cJSON_Number=8
(double)INT_MAX=2147483647.0   (double)INT_MIN=-2147483648.0
```

So the two saturation thresholds the Rust compares against
(`INT_MAX as c_double`, `INT_MIN as c_double`) are bit-exact matches for the C's
`INT_MAX` (int, promoted) and `(double)INT_MIN`, and both are exactly
representable in binary64 — the comparison introduces no rounding of its own.

Consequently the `else` branch is only reachable with
`-2147483648.0 < number < 2147483647.0`, where `(int)number` in C and
`number as c_int` in Rust are both exact truncation toward zero. Rust's `as`
saturation semantics therefore never come into play, and NaN cannot reach it
either: `strtod` can only return NaN for `nan`/`nan(...)` spellings, whose
letters (`n`, `a`) are filtered out by the scanner's `default:` arm before
`strtod` is ever called.

## Sensitivity check (mutation testing)

A suite that passes proves nothing unless it can fail. `mutate.sh` plants 22
deliberate bugs in `src/lib.rs`, rebuilds, and re-runs:

* **19 / 19 behaviour-changing mutations were caught**, including: dropping
  `'E'`, `'+'` or `'.'` from the accepted set; leaking `'x'` (which would let
  libc `strtod` parse hex floats) or `' '` (whitespace); advancing `offset` by
  the scan length instead of `after_end - number_c_string`; removing the
  `content == NULL` check; `cJSON_Number = 1<<4`; returning `true` on the parse
  error; `<= length` in the scan bound; `malloc` one byte short; either
  saturation bound off by one; flooring instead of truncating toward zero;
  omitting the `valueint` or `type` write; writing `valuedouble` into the
  `valueint` slot; omitting the NUL terminator; and — as a permanent regression
  guard for the bug found here — reverting `item_store!` to a place-expression
  store.
* **3 mutations were correctly *not* caught** because they are provably
  unobservable, which is a proof rather than a gap:
  1. `>=` → `>` at the `INT_MAX` bound: at exactly `2147483647.0` the `else`
     branch's `(int)number` yields `INT_MAX` anyway.
  2. `<=` → `<` at the `INT_MIN` bound: same argument at `-2147483648.0`.
  3. Never setting `has_decimal_point`: the guarded loop replaces `'.'` with
     `decimal_point`, which the C hard-codes to `'.'` — a no-op.

`src/lib.rs` was restored byte-for-byte afterwards (every mutation site
re-verified present in its original form, file 251 lines).

The M1 regression guard was also checked in the other direction: reverting
`item_store!(item, valuedouble, number)` to `(*item).valuedouble = number` makes
`m1_item_null_produces_the_same_fatal_signal` fail with
`C uses signal Some(11), Rust Some(6)` — so the test genuinely detects the bug it
was written for.

## Quirks of the C that the Rust correctly preserves

These look like bugs but are the C's real behaviour, so they are replicated, not
fixed:

1. **No `item == NULL` check.** `lib.c:92` writes `item->valuedouble`
   unconditionally. The Rust does the same unchecked write, and this *is* covered
   by a differential test: `m1_item_null_produces_the_same_fatal_signal` re-execs
   itself once per implementation and compares the fatal signal, exit code and
   stderr. Both raise SIGSEGV at `si_addr = 0x8` with empty stderr. This is the
   test that found the one real divergence (`ERRORS.md` M1).
2. **`decimal_point` is hard-coded to `'.'`,** so the "replace `.` with the
   locale's decimal point" loop is a no-op. Real cJSON queries `localeconv()`;
   this extract does not.
3. **`"0x10"` parses as `0`, consuming one byte.** The scanner filters `x` out
   before `strtod` is called, so libc's hex-float path is unreachable. Same for
   `"inf"`, `"nan"`, `"1_000"` and leading whitespace — all rejected by the
   scanner, never by `strtod`.
4. **Saturation compares against `INT_MAX`/`INT_MIN` promoted to `double`,** so
   `"2147483646.999999999"` saturates to `INT_MAX`: it is not representable in
   binary64 (the spacing near 2³¹ is ≈ 4.8e-7) and rounds up to exactly
   `2147483647.0`, tripping the `>=` branch.
5. **`length`, not a NUL byte, bounds the scan.** Bytes past `length` are never
   read even when they are valid number characters.
6. **`offset` advances by `after_end - number_c_string`, not by the scan
   length,** so `"1e"` consumes only `1` byte while scanning `2`.
