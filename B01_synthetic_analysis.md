# B01_synthetic — Per-Case Analysis

Comparing HARVEST e2e (GPT-codex-5.3) vs kiro-cli on cases where results differ.

## Summary

| Metric | HARVEST e2e | kiro-cli |
|---|---|---|
| Cases built | 83/85 | 85/85 |
| Cases 100% pass | 63/83 | 82/83 |
| Test vectors passed | 348/412 (84.5%) | 392/393 (99.7%) |

## Root cause categories

- **BIN_NAME**: HARVEST missing `[[bin]] name = "driver"` in Cargo.toml — test runner can't find binary
- **LIB_NAME**: Library `.so` name doesn't match what cando2 runner expects
- **BUILD_FAIL**: Translation doesn't compile
- **SEMANTIC**: Translation compiles but produces wrong output
- **STDIN**: Differences in stdin/scanf handling
- **UB**: Test involves undefined behavior (skipped by test runner)

## Analyzed cases

### 027_ctype_ascii — HARVEST: 0/21, kiro: 21/21
- **Root cause**: BIN_NAME
- **Details**: Both translations are nearly identical — they both use `libc::isalnum()` etc. via FFI. The translations produce correct, byte-identical output. But HARVEST's Cargo.toml has `name = "_027_ctype_ascii"` with no `[[bin]]` section, so the binary is named `_027_ctype_ascii` instead of `driver`. The test runner can't find it → 0/21.
- **kiro-cli fix**: The prompt explicitly says `[[bin]] name = "driver"`, and the agent verifies the build works.

### 042_float_union — HARVEST: 0/5, kiro: 5/5
- **Root cause**: BIN_NAME
- **Details**: Same as 027. Both translations use `libc::printf` with `%llx %a %.4f` format and `f64::to_bits()` for the union. Logic is essentially identical. HARVEST's Cargo.toml missing `[[bin]] name = "driver"`.

### 007_errno-pow — HARVEST: 0/12 (build fail), kiro: 12/12
- **Root cause**: BUILD_FAIL
- **Details**: HARVEST calls `libc::pow(base, exponent)` — but the Rust `libc` crate does not export `pow`. It only exposes OS-level C bindings, not libm math functions. This produces `error[E0425]: cannot find function 'pow' in crate 'libc'`. Since the binary never compiles, all 12 vectors fail. HARVEST also uses `edition = "2024"` (not yet stable). kiro-cli avoids this by declaring `pow` via manual `extern "C"` FFI (`extern "C" { fn pow(base: f64, exp: f64) -> f64; }`), linking directly against the system's libm. It also avoids the `errno` crate by calling `libc::__errno_location()` directly.

### 009_stack_buffer_overflow — HARVEST: 4/8, kiro: 8/8
- **Root cause**: SEMANTIC
- **Details**: HARVEST has two distinct semantic bugs:
  1. **EOF detection**: Uses `read_line().is_ok()` to emulate C's `fgets()`. But `read_line` returns `Ok(0)` at EOF — still `is_ok() == true`. When stdin has no trailing newline (test02: `"2"`) or is empty (test03), HARVEST treats EOF as a successful read of empty string → `parse` returns 0 → wrong code path. kiro-cli implements byte-level `fgets()` that returns `None` on EOF.
  2. **Integer overflow in atoi**: Uses `parse::<i32>().unwrap_or(0)`. For `"9000000000"` (exceeds i32), parse fails → `unwrap_or(0)` → data=0, which passes bounds check incorrectly. kiro-cli implements `c_atoi()` with `wrapping_mul`/`wrapping_add` to reproduce C's wrapping overflow.

  | Test | Input | HARVEST | kiro | Failure reason |
  |------|-------|---------|------|----------------|
  | test01 | `2\n5` | ✅ | ✅ | — |
  | test02 | `2` (no newline) | ❌ | ✅ | EOF not detected by `read_line` |
  | test03 | `` (empty) | ❌ | ✅ | EOF not detected by `read_line` |
  | test04 | `-2\n5` | ✅ | ✅ | — |
  | test05 | `-2\n-5` | ✅ | ✅ | — |
  | test06 | `200\n5` | ✅ | ✅ | — |
  | test07 | `200\n200` | ❌ | ✅ | has_ub — panic vs UB |
  | test08 | `9000000000\n5` | ❌ | ✅ | `parse::<i32>` fails on overflow |

### 019_integer_overflow_char_max_multiply — HARVEST: 1/2, kiro: 2/2
- **Root cause**: BIN_NAME
- **Details**: HARVEST's Cargo.toml has no `[[bin]]` section → binary named `_019_integer_overflow_char_max_multiply` instead of `driver`. Semantically both translations are correct and produce identical output for both vectors. The fix is trivially adding `[[bin]] name = "driver"`.

### 030_mutable_buffer_overlap_extrahard — HARVEST: 7/8, kiro: 8/8
- **Root cause**: SEMANTIC (has_ub test handling)
- **Details**: Both translations produce correct output for all 6 non-UB vectors. The failing vector is test04 (`stdin: "2147483647\n500"`, `has_ub: "Overflow"`). HARVEST's own test runner does NOT skip `has_ub` tests — it runs them and expects empty stdout. The translation produces wrapping arithmetic output in release mode, causing mismatch. kiro-cli's `runtests` runner skips `has_ub` vectors entirely. Additionally, HARVEST has the BIN_NAME issue (no `[[bin]]` section), but its own runner finds the binary by package name.

## Remaining cases (not yet analyzed)

### 003_string_slicing — HARVEST: 11/12, kiro: 12/12
- **Root cause**: TODO

### 010_integer_overflow — HARVEST: 3/4, kiro: 4/4
- **Root cause**: TODO

### 010_integer_overflow_lib — HARVEST: 2/3, kiro: 3/3
- **Root cause**: TODO

### 011_uninit_char_ptr_lib — HARVEST: 1/2, kiro: 2/2
- **Root cause**: TODO

### 012_uninit_int_ptr_lib — HARVEST: 1/2, kiro: 2/2
- **Root cause**: TODO

### 015_return_stack_buffer — HARVEST: 1/2, kiro: 2/2
- **Root cause**: TODO

### 015_return_stack_buffer_lib — HARVEST: 1/2, kiro: 2/2
- **Root cause**: TODO

### 016_divide_by_zero_float — HARVEST: 3/4, kiro: 4/4
- **Root cause**: TODO

### 016_divide_by_zero_float_lib — HARVEST: 3/4, kiro: 4/4
- **Root cause**: TODO

### 019_integer_overflow_char_max_multiply_lib — HARVEST: 1/2, kiro: 2/2
- **Root cause**: TODO

### 030_mutable_buffer_overlap_extrahard_lib — HARVEST: 5/6, kiro: 6/6
- **Root cause**: TODO

## Cases where both fail

### 002_stdin_echo — HARVEST: 3/4, kiro: 3/4
- **Root cause**: STDIN
- **Details**: The failing vector sends a NUL byte via stdin. Both translations fail to reproduce C's exact behavior for NUL byte handling in `getchar()`/`putchar()` loops.

## Cases skipped (undefined behavior)

### 008_long_run / 008_long_run_lib — both skipped
- **Root cause**: UB — test vectors marked with `has_ub`, skipped by test runner

## Key takeaways

1. **BIN_NAME is the #1 issue**: Many HARVEST "failures" are actually correct translations with the wrong binary name. Adding `[[bin]] name = "driver"` would fix ~10 cases instantly.

2. **BUILD_FAIL from missing FFI**: HARVEST's use of `libc::pow()` (which doesn't exist in the `libc` crate) is a single-shot mistake that an agentic approach catches immediately via `cargo build`.

3. **Subtle C semantics**: The real translation quality differences show up in edge cases like `fgets()` EOF handling and `atoi()` integer overflow wrapping — things that require understanding C's exact behavior, not just the "obvious" Rust equivalent.

4. **Test runner differences**: HARVEST's own runner and the standard `runtests` runner handle `has_ub` tests differently, which affects scoring.
