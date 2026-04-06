# B01_synthetic — Per-Case Analysis

Comparing HARVEST e2e (GPT-codex-5.3) vs kiro-cli on cases where results differ.

## Summary

| Metric | HARVEST e2e | kiro-cli |
|---|---|---|
| Cases built | 83/85 | 85/85 |
| Cases 100% pass | 63/83 | 82/83 |
| Test vectors passed | 348/412 (84.5%) | 392/393 (99.7%) |

## Root cause breakdown

| Root Cause | Count | Cases |
|---|---|---|
| BIN_NAME | 5 | 027_ctype_ascii, 042_float_union, 019_integer_overflow_char_max_multiply, 010_integer_overflow, (latent in many others) |
| LIB_NAME | 3 | 010_integer_overflow_lib, 011_uninit_char_ptr_lib, 012_uninit_int_ptr_lib |
| BUILD_FAIL | 1 | 007_errno-pow |
| SEMANTIC | 2 | 009_stack_buffer_overflow, 003_string_slicing |
| UB_HANDLING | 6 | 015_return_stack_buffer, 015_return_stack_buffer_lib, 016_divide_by_zero_float, 016_divide_by_zero_float_lib, 019_integer_overflow_char_max_multiply_lib, 030_mutable_buffer_overlap_extrahard_lib |
| UB_HANDLING + SEMANTIC | 1 | 030_mutable_buffer_overlap_extrahard |
| STDIN | 1 | 002_stdin_echo (both fail) |

## All analyzed cases

### 027_ctype_ascii — HARVEST: 0/21, kiro: 21/21
- **Root cause**: BIN_NAME
- **Details**: Both translations are nearly identical — they both use `libc::isalnum()` etc. via FFI and produce correct output. HARVEST's Cargo.toml has `name = "_027_ctype_ascii"` with no `[[bin]]` section → binary not named `driver` → test runner can't find it → 0/21.

### 042_float_union — HARVEST: 0/5, kiro: 5/5
- **Root cause**: BIN_NAME
- **Details**: Same as 027. Both use `libc::printf` with `%llx %a %.4f` and `f64::to_bits()`. Logic identical. Missing `[[bin]] name = "driver"`.

### 007_errno-pow — HARVEST: 0/12 (build fail), kiro: 12/12
- **Root cause**: BUILD_FAIL
- **Details**: HARVEST calls `libc::pow()` — but the Rust `libc` crate doesn't export `pow` (only OS-level bindings, not libm). Fatal: `error[E0425]: cannot find function 'pow' in crate 'libc'`. kiro-cli declares `pow` via manual `extern "C"` FFI, linking directly against system libm. Also avoids the `errno` crate by using `libc::__errno_location()` directly.

### 009_stack_buffer_overflow — HARVEST: 4/8, kiro: 8/8
- **Root cause**: SEMANTIC
- **Details**: Two distinct bugs in HARVEST:
  1. **EOF detection**: `read_line().is_ok()` doesn't detect EOF (returns `Ok(0)` at EOF). kiro-cli implements byte-level `fgets()` returning `None` on EOF.
  2. **atoi overflow**: `parse::<i32>().unwrap_or(0)` for `"9000000000"` → parse fails → 0 instead of wrapping. kiro-cli uses `wrapping_mul`/`wrapping_add` to reproduce C's wrapping behavior.

### 003_string_slicing — HARVEST: 11/12, kiro: 12/12
- **Root cause**: SEMANTIC
- **Details**: The C source has a subtle bug: `strtol(argv[3], NULL, 10)` passes NULL for endptr, so the `end` variable retains its stale value from the `argv[2]` parse. The check `if (end == argv[3])` is dead code. When `"asdf"` is passed as argv[3], strtol returns 0, the dead branch is skipped, and `stop <= start` triggers `"Error: stop must come after start!"`. HARVEST uses `parse_i64().ok()` which returns `None` for `"asdf"`, making the integer-check branch live — it incorrectly prints `"Third argument must be an integer!"`. kiro-cli faithfully reproduces the stale-pointer bug.

### 010_integer_overflow — HARVEST: 3/4, kiro: 4/4
- **Root cause**: BIN_NAME
- **Details**: HARVEST binary = `_010_integer_overflow` (missing `[[bin]] name = "driver"`). exec_runner.py hardcodes `bin_dir / "driver"` → can't find binary. Both translations produce correct output.

### 010_integer_overflow_lib — HARVEST: 2/3, kiro: 3/3
- **Root cause**: LIB_NAME
- **Details**: HARVEST .so = `lib_010_integer_overflow_lib.so` (`[lib] name = "_010_integer_overflow_lib"`). cando2 harness expects `library: "driver"` → `libdriver.so` → Library::new() panics. kiro-cli uses `[package] name = "driver"` → `libdriver.so`.

### 011_uninit_char_ptr_lib — HARVEST: 1/2, kiro: 2/2
- **Root cause**: LIB_NAME
- **Details**: Same as 010_lib. HARVEST .so = `lib_011_uninit_char_ptr_lib.so`, runner expects `libdriver.so`.

### 012_uninit_int_ptr_lib — HARVEST: 1/2, kiro: 2/2
- **Root cause**: LIB_NAME
- **Details**: Same as 010_lib. HARVEST .so = `lib_012_uninit_int_ptr_lib.so`, runner expects `libdriver.so`.

### 015_return_stack_buffer — HARVEST: 1/2, kiro: 2/2
- **Root cause**: UB_HANDLING
- **Details**: 2 vectors (good + bad/has_ub). HARVEST's runner runs both → 1/2. Standard `runtests` runner skips `has_ub` → 2/2. Both translations produce correct output for the non-UB vector. Latent BIN_NAME issue also present.

### 015_return_stack_buffer_lib — HARVEST: 1/2, kiro: 2/2
- **Root cause**: UB_HANDLING
- **Details**: Same pattern. cando2 runner doesn't handle `has_ub` — panics on `Option::unwrap()` when `lib_state_out` is missing. Latent LIB_NAME issue also present.

### 016_divide_by_zero_float — HARVEST: 3/4, kiro: 4/4
- **Root cause**: UB_HANDLING
- **Details**: 4 vectors (test01-03 + test04/has_ub). HARVEST runs all → 3/4. Standard runner skips test04 → 4/4. Both translations correct for non-UB vectors. Latent BIN_NAME issue.

### 016_divide_by_zero_float_lib — HARVEST: 3/4, kiro: 4/4
- **Root cause**: UB_HANDLING
- **Details**: Same pattern as executable variant. Latent LIB_NAME issue.

### 019_integer_overflow_char_max_multiply — HARVEST: 1/2, kiro: 2/2
- **Root cause**: BIN_NAME
- **Details**: Missing `[[bin]] name = "driver"`. Both translations semantically correct and produce identical output.

### 019_integer_overflow_char_max_multiply_lib — HARVEST: 1/2, kiro: 2/2
- **Root cause**: UB_HANDLING
- **Details**: Failing test is `bad.json` (has_ub — integer overflow CHAR_MAX * 2). cando2 runner doesn't handle has_ub: calls `tc.equals_expected()` which unwraps `lib_state_out`, but bad.json has no `lib_state_out` → panics. Python runner skips has_ub before invoking cando2. Latent LIB_NAME issue (HARVEST uses directory name instead of `"driver"`).

### 030_mutable_buffer_overlap_extrahard — HARVEST: 7/8, kiro: 8/8
- **Root cause**: UB_HANDLING + SEMANTIC
- **Details**: Failing vector is test04 (`has_ub: "Overflow"`). HARVEST's runner doesn't skip has_ub tests — runs them and expects empty stdout. Translation produces wrapping arithmetic output → mismatch. Standard runner skips has_ub → 8/8. Both translations correct for all non-UB vectors. Latent BIN_NAME issue.

### 030_mutable_buffer_overlap_extrahard_lib — HARVEST: 5/6, kiro: 6/6
- **Root cause**: UB_HANDLING
- **Details**: Failing test is test06.json (has_ub, contains value 2147483647000 which overflows i32). cando2 runner tries to deserialize into `State { data: Vec<c_int> }` but 2147483647000 doesn't fit → serde panics. Python runner skips has_ub tests. Latent LIB_NAME issue.

## Cases where both fail

### 002_stdin_echo — HARVEST: 3/4, kiro: 3/4
- **Root cause**: STDIN
- **Details**: The failing vector sends a NUL byte via stdin. Both translations fail to reproduce C's exact behavior for NUL byte handling in `getchar()`/`putchar()` loops.

## Cases skipped (undefined behavior)

### 008_long_run / 008_long_run_lib — both skipped
- **Root cause**: UB — test vectors marked with `has_ub`, skipped by test runner

## Key takeaways

1. **BIN_NAME / LIB_NAME is the #1 issue (8 cases)**: Many HARVEST "failures" are correct translations with wrong artifact names. Adding `[[bin]] name = "driver"` or matching the CMake library name would fix these instantly.

2. **UB_HANDLING scoring difference (6 cases)**: HARVEST's own test runner doesn't skip `has_ub` tests, while the standard `runtests` runner does. This inflates the gap — both translations are equally correct on non-UB inputs.

3. **BUILD_FAIL (1 case)**: `libc::pow()` doesn't exist in the Rust `libc` crate. An agentic approach catches this immediately via `cargo build`.

4. **Genuine SEMANTIC differences (2 cases)**: Only `009_stack_buffer_overflow` (fgets EOF + atoi overflow) and `003_string_slicing` (stale pointer / dead code faithfulness) represent real translation quality differences where kiro-cli's agentic iteration produced better C-semantics fidelity.

5. **Apples-to-oranges scoring**: If HARVEST were scored with the same `runtests` runner (skipping has_ub) AND had correct artifact names, its score would be significantly higher — likely ~81/83 instead of 63/83. The real translation quality gap is narrow (2 semantic cases).
