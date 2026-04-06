# C-to-Rust Translation Failure Analysis Report

**Date:** 2026-03-18
**Project:** harvest-agentic (DARPA TRACTOR)
**Scope:** Automated fix pass over failing test vectors across all batteries

---

## 1. Executive Summary

An automated fix pass was run against 100 failing test cases across 6 batteries. Of these, 85 had fix patches generated and 15 either needed no fix or could not be patched.

| Metric | Count |
|---|---|
| Total cases analyzed | 100 |
| Cases fixed (all vectors pass) | **83** |
| Cases still failing | **17** |
| Fix patches generated | 85 |
| Cases that passed without any fix | 8 |
| Cases where no fix was attempted and still failing | 7 |
| Test vectors passed (total) | 801 |
| Test vectors still failing | 4 |

**Overall fix success rate: 83%** (83/100 cases fully passing after fix).

Of the 85 cases where a patch was generated, **76 were fully fixed** (89% patch success rate). The remaining 10 patched cases still fail due to additional unfixed issues beyond what the patch addressed.

---

## 2. Per-Battery Breakdown

| Battery | Cases | Fixed | Still Failing | No Fix Needed | Vectors Passed | Vectors Failed |
|---|---|---|---|---|---|---|
| B01_organic | 6 | 6 | 0 | 2 | 100 | 0 |
| B01_synthetic | 1 | 1 | 0 | 0 | 4 | 0 |
| B02_organic | 4 | 4 | 0 | 0 | 20 | 0 |
| B02_synthetic | 13 | 13 | 0 | 4 | 618 | 0 |
| P00_perlin_noise | 0 | — | — | — | — | — |
| P01_sphincs_plus | 76 | 59 | 17 | 9 | 59 | 4 |
| **Total** | **100** | **83** | **17** | **15** | **801** | **4** |

All B01/B02 battery cases were fully fixed. All 17 remaining failures are in P01_sphincs_plus.

---

## 3. Root Cause Analysis

Fixes are grouped by the type of bug they corrected. Many cases required multiple fixes (e.g., a SPHINCS+ case might need both a missing FFI export and a bytes-vs-bits correction), so a single case may appear in multiple categories.

### 3.1 Missing FFI Exports — 48 cases (41 fixed, 7 still failing)

**The single most common bug class.** The C test harness calls functions via specific symbol names (e.g., `SPX_prf_addr`, `SPX_gen_message_random`, `SPX_hash_message`), but the translator produced internal Rust functions without `#[no_mangle] pub extern "C"` wrappers.

The fix adds a thin FFI shim that converts raw pointers to slices and delegates to the internal implementation:

```rust
// Added by fix:
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(
    out: *mut u8, ctx: *const SpxCtx, addr: *const [u32; 8],
) {
    let out = std::slice::from_raw_parts_mut(out, SPX_N);
    hash_blake::prf_addr(out, &*ctx, &*addr);
}
```

**Affected batteries:** P01_sphincs_plus exclusively (all 48 cases).

### 3.2 Blake Hash Bytes-vs-Bits Confusion — 26 cases (21 fixed, 5 still failing)

The C BLAKE implementation's `blake256_update`/`blake512_update` functions accept a byte count, but the translator incorrectly multiplied by 8, passing a bit count instead. This caused the hash to process 8× too much data.

```diff
-    blake256_update(&mut s, &sk_prf[..SPX_N], (SPX_N as u64) * 8);
+    blake256_update(&mut s, &sk_prf[..SPX_N], SPX_N as u64);
```

This is a semantic misunderstanding of the C API — the parameter name `datalen` was ambiguous, and the translator assumed bits when the implementation expected bytes.

### 3.3 OpenSSL-to-Pure-Rust AES Replacement — 11 cases (all fixed)

The C code uses OpenSSL's `EVP_EncryptUpdate` for AES-256-ECB in the deterministic RNG (DRBG). The translator initially produced Rust code that called OpenSSL via FFI, which failed to link in the test environment. The fix replaces this with the pure-Rust `aes` crate:

```diff
-use openssl::symm::{Cipher, Crypter, Mode};
+use aes::cipher::{BlockEncrypt, KeyInit};
+use aes::Aes256;
```

### 3.4 stdin/fgets Emulation — 7 cases (all fixed)

Multiple B02 cases used `BufRead::read_line()` or `lines()` for stdin, which doesn't match C's `fgets()` semantics. Key differences:
- `fgets` reads at most N-1 bytes and stops at newline (included)
- `fgets` leaves excess bytes in the buffer for the next call
- `BufRead::lines()` consumes the entire line regardless of buffer size

The fix replaces `BufRead` with byte-at-a-time `Read::read()` to faithfully emulate `fgets`:

```rust
fn fgets_256(stdin: &mut dyn io::Read) -> Option<Vec<u8>> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    for _ in 0..255 {
        match stdin.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => { buf.push(byte[0]); if byte[0] == b'\n' { break; } }
            Err(_) => break,
        }
    }
    if buf.is_empty() { None } else { Some(buf) }
}
```

**Affected cases:** `generic-foreach`, `mutable-duplication-dag`, `pointer-comparison-ascii-art`, `static-vars-fpts`, `strcmp`, `underhanded-c-luggage`, `002_stdin_echo`.

### 3.5 Wrong FFI Symbol Name — 4 cases (all fixed)

The translator used the internal function name (e.g., `initialize_hash_function`, `prf_addr`) instead of the namespaced C symbol (e.g., `SPX_initialize_hash_function`, `SPX_prf_addr`). The C code uses a `SPX_NAMESPACE` macro that prepends `SPX_` to all public symbols.

```diff
-pub extern "C" fn initialize_hash_function(ctx: *mut SpxCtx) {
+pub extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
```

### 3.6 Missing `#[repr(C)]` on Structs — 4 cases (all fixed)

Structs passed across the FFI boundary lacked `#[repr(C)]`, causing field layout mismatches:

```diff
+#[repr(C)]
 pub struct SpxCtx {
     pub pub_seed: [u8; SPX_N],
     pub sk_seed: [u8; SPX_N],
 }
```

### 3.7 Blake Buffer Handling Logic — 3 cases (2 fixed, 1 still failing)

The `blake256_update` function has a `left` variable tracking buffered bytes. After consuming the buffer, the C code implicitly resets `left = 0` by falling through. The translator failed to reset `left`, causing subsequent data to be written at the wrong buffer offset:

```diff
-    let left = (s.buflen >> 3) as usize;
+    let mut left = (s.buflen >> 3) as usize;
     ...
         data_off += fill;
         datalen -= (fill as u64) << 3;
-        // left = 0 implicitly
+        left = 0;
```

### 3.8 Wrong C Macro Expansion — 2 cases (all fixed)

The `macrodepth_mul_4` and `macrodepth_sub_6` cases use C preprocessor macros to generate operator-specific code (`DEFINE_ACCUM(mul)`, `DEFINE_ACCUM(sub)`). The translator expanded all macros as addition instead of the correct operator:

```diff
-fn op_add(a: i32, b: i32) -> i32 { a + b }
+fn op_mul(a: i32, b: i32) -> i32 { a * b }
```

### 3.9 Float/Integer Precision Issues — 4 cases (all fixed)

Several distinct sub-patterns:

- **Cast ordering** (`contrast_ratio_lib`): `(r + 0.055) as f64` performs f32 addition then widens; the C code does `(double)(r) + 0.055` which widens first. Fix: `(r as f64 + 0.055)`.
- **Integer widening before wrapping arithmetic** (`hex2bin_lib`): `c_num.wrapping_sub(10) as u32` wraps in u8 then widens; C does `(uint32_t)(c_num - 10)` which wraps in u32. Fix: `(c_num as u32).wrapping_sub(10)`.
- **Modulo semantics** (`hsl_to_rgb_lib`): `rem_euclid(2.0)` always returns non-negative; C's `fmod` can return negative. Fix: use `% 2.0`.
- **Float literal type** (`synth_pair_lib`): `0.5` defaulted to f64; needed `0.5_f32`.

### 3.10 NUL Byte / C String Semantics — 2 cases (all fixed)

Rust strings can contain NUL bytes; C strings terminate at NUL. The translator didn't truncate at NUL when passing data to functions that expect C-string semantics:

```diff
+    let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
+    let result = process_decisions(&bytes[..len], operation, param);
```

### 3.11 scanf/sscanf Emulation — 3 cases (all fixed)

C's `scanf("%d")` skips leading whitespace and parses integers with specific overflow behavior. The translator used Rust's `str::parse()` which has different semantics. Fixes added custom `sscanf_int` / `scan_int_with_ws` functions.

### 3.12 Pointer Arithmetic / C Macro Indexing — 1 case (fixed)

In `intput_lib`, the C macro `hmget` indexes from the hash pointer `t`, not from `arr = t - 1`. The translator indexed from `arr`:

```diff
-    let entry = arr.add(INTMAP_ELEMSIZE * idx as usize) as *mut IntMapEntry;
+    let entry = (*intmap).add(INTMAP_ELEMSIZE * idx as usize) as *mut IntMapEntry;
```

### 3.13 Other Single-Instance Fixes

| Bug Type | Case | Description |
|---|---|---|
| Integer type mismatch | `memmove` | Parsed `u32` instead of `i64` for byte values that can be negative in C |
| Spurious guard condition | `static-vars-fpts` | Added `pattern.is_empty()` check not present in C |
| Spurious null check | `mutable-duplication-dag` | Added empty-string check in `add_node` not present in C |
| int→size_t cast | `strcmp` | `n as usize` vs `n as u32 as usize` for wrapping negative-to-large conversion |
| Missing driver module | `cJSON_lib` | Entire `driver.rs` file was missing from translation |
| Major logic rewrite | `arr_ins_lib` | Hash map stub was incorrect; replaced with direct array insertion logic |
| Keccak round logic | `030_shake_128f_robust` | Variable shadowing errors in unrolled Keccak-f permutation rounds |
| Missing test harness label | `048_haraka_192s_robust` | `absorb_label("sm")` call missing before `absorb_bytes` in KAT harness |

---

## 4. Common Patterns and Prompt Improvement Recommendations

### 4.1 FFI Export Awareness

**Problem:** 48 of 100 cases (48%) required adding missing FFI exports. The translator produces correct internal logic but fails to expose it with the right C-callable symbol names.

**Recommendation:** The translation prompt should explicitly instruct:
> "For every function declared in the C header files (`.h`), ensure a corresponding `#[unsafe(no_mangle)] pub extern "C" fn` exists in `lib.rs` with the exact symbol name used by the C code, including any namespace prefix macros like `SPX_NAMESPACE`."

### 4.2 Hash API Byte/Bit Convention

**Problem:** 26 cases had the same bytes-vs-bits confusion in BLAKE hash calls.

**Recommendation:** Add to the prompt:
> "When translating hash function calls, carefully check whether length parameters represent bytes or bits. The C BLAKE reference implementation uses byte counts despite the parameter being named `datalen`. Do not multiply by 8 unless the C code explicitly does so."

### 4.3 C stdio Semantics (fgets, scanf)

**Problem:** 10+ cases needed custom `fgets`/`scanf` emulation because Rust's `BufRead` doesn't match C's buffered I/O semantics.

**Recommendation:**
> "When translating C programs that use `fgets()`, do NOT use Rust's `BufRead::read_line()` or `lines()`. Instead, implement byte-at-a-time reading with `Read::read()` that stops at newline and respects the buffer size limit. Similarly, `scanf("%d")` requires custom parsing that matches C's whitespace-skipping and overflow behavior."

### 4.4 `#[repr(C)]` on FFI Structs

**Problem:** 4 cases had structs crossing the FFI boundary without `#[repr(C)]`.

**Recommendation:**
> "Any struct that is passed to or from C code (via pointers or by value) MUST have `#[repr(C)]` to ensure field layout matches the C ABI."

### 4.5 C Preprocessor Macro Expansion

**Problem:** 2 cases had macros expanded with the wrong operator.

**Recommendation:**
> "When C code uses token-pasting or parameterized macros (e.g., `DEFINE_ACCUM(mul)`), ensure the Rust translation uses the correct operator for each instantiation. Do not copy-paste from one expansion to another."

### 4.6 Integer Promotion and Wrapping Arithmetic

**Problem:** Multiple cases had incorrect cast ordering — Rust's `as` casts happen at the point of the expression, while C's implicit integer promotion widens before arithmetic.

**Recommendation:**
> "In C, integer arithmetic on types smaller than `int` is performed after implicit promotion to `int` (or `unsigned int`). When translating wrapping/bitwise arithmetic on `u8`/`i8`/`u16`/`i16`, cast to the wider type BEFORE the operation, not after."

### 4.7 OpenSSL Dependencies

**Problem:** 11 cases used OpenSSL FFI that didn't link in the test environment.

**Recommendation:**
> "Do not use OpenSSL crate bindings. For AES-256-ECB, use the pure-Rust `aes` crate (`aes::Aes256` with `BlockEncrypt`). For SHA-256, use the `sha2` crate."

---

## 5. Cases That Could Not Be Fixed

17 cases remain failing, all in P01_sphincs_plus. They fall into two groups:

### 5.1 Patched But Still Failing — 10 cases

These cases received patches (typically adding missing FFI exports and fixing bytes-vs-bits), but additional issues remain that the automated fix pass could not resolve. Common residual problems include:
- Additional unfixed blake buffer logic errors beyond the `left` variable reset
- Incorrect thash (tree hash) robust-mode XOR masking logic
- SHA-2 padding or MGF1 implementation bugs in the translated code

| Case | Vectors Passed | Vectors Failed | Patch Applied |
|---|---|---|---|
| `006_blake_128f_robust` | 0 | 1 | bytes-vs-bits fix |
| `008_blake_128s_robust` | 0 | 0 | blake buffer + bytes-vs-bits |
| `009_blake_192f_simple` | 0 | 0 | bytes-vs-bits fix |
| `057_blake_128f_r_initialize_hash_function_lib` | 0 | 0 | missing FFI export |
| `063_blake_128s_s_gen_message_random_lib` | 0 | 0 | bytes-vs-bits + FFI export |
| `064_blake_128s_s_hash_message_lib` | 0 | 0 | bytes-vs-bits + FFI export |
| `090_blake_256f_r_prf_addr_lib` | 0 | 0 | missing FFI export |
| `102_sha2_128f_s_prf_addr_lib` | 0 | 0 | missing FFI export |
| `104_sha2_128f_s_hash_message_lib` | 0 | 0 | missing FFI export |
| `118_sha2_192f_s_prf_addr_lib` | 0 | 0 | missing FFI export |

The 0 vectors passed / 0 vectors failed pattern indicates the test harness could not even run (likely a link error or crash before any test vector was evaluated), suggesting additional missing exports or build issues beyond what was patched.

### 5.2 No Fix Attempted — 7 cases

These cases were not patched at all, likely because the automated fixer could not determine a viable fix within its budget:

| Case | Vectors Passed | Vectors Failed |
|---|---|---|
| `112_sha2_128s_s_hash_message_lib` | 0 | 1 |
| `116_sha2_128s_r_hash_message_lib` | 0 | 1 |
| `119_sha2_192f_s_gen_message_random_lib` | 0 | 0 |
| `121_sha2_192f_r_initialize_hash_function_lib` | 0 | 0 |
| `127_sha2_192s_s_gen_message_random_lib` | 0 | 0 |
| `130_sha2_192s_r_prf_addr_lib` | 0 | 1 |
| `131_sha2_192s_r_gen_message_random_lib` | 0 | 0 |

These are all SHA-2 variant library cases. The SHA-2 hash implementation in SPHINCS+ involves HMAC-SHA256, SHA-256 MGF1, and address compression — more complex than the BLAKE variants. The translated SHA-2 code likely has deeper implementation bugs (e.g., incorrect padding, wrong endianness in message schedule) that require more extensive manual review.

---

## 6. Summary of Fix Categories

| Root Cause Category | Cases | Fixed | Still Failing |
|---|---|---|---|
| Missing FFI exports | 48 | 41 | 7 |
| Blake bytes-vs-bits confusion | 26 | 21 | 5 |
| OpenSSL → pure Rust AES | 11 | 11 | 0 |
| stdin/fgets emulation | 7 | 7 | 0 |
| Wrong FFI symbol name | 4 | 4 | 0 |
| Missing `#[repr(C)]` | 4 | 4 | 0 |
| Blake buffer handling | 3 | 2 | 1 |
| scanf emulation | 3 | 3 | 0 |
| BufRead → Read | 3 | 3 | 0 |
| Wrong macro expansion | 2 | 2 | 0 |
| NUL byte handling | 2 | 2 | 0 |
| Float/int precision | 4 | 4 | 0 |
| Pointer arithmetic | 1 | 1 | 0 |
| Other (single-instance) | 7 | 7 | 0 |
| **Unfixed (no patch)** | **7** | **0** | **7** |

> Note: Cases often have multiple root causes, so column totals exceed 100.

---

## 7. Key Takeaways

1. **FFI boundary issues dominate.** Nearly half of all failures were caused by missing or incorrectly named C-callable exports. This is the single highest-leverage area for prompt improvement.

2. **API semantic misunderstanding is the second biggest class.** The bytes-vs-bits confusion in BLAKE affected 26 cases — a single conceptual error replicated across many translation instances.

3. **C stdio semantics are a recurring trap.** Rust's standard I/O abstractions (`BufRead`, `lines()`) don't match C's `fgets`/`scanf` behavior. A reusable `fgets` emulation helper should be part of the translation toolkit.

4. **All B01/B02 failures were fixable.** The general-purpose C-to-Rust translation is solid; failures are concentrated in the cryptographic library (P01_sphincs_plus) where precise bit-level correctness is required.

5. **SHA-2 variants are the hardest to fix.** All 7 unfixable cases are SHA-2 SPHINCS+ library functions, suggesting the SHA-2 hash translation has deeper issues than BLAKE.
