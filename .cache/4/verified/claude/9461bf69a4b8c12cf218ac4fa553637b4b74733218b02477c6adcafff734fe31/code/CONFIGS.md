# CONFIGS.md — configuration-surface table (Phase B)

## Build-time configuration axes

* `Cargo.toml`: **no `[features]` table** → exactly one feature combination (the
  empty set). `cargo …` and `cargo … --no-default-features` are equivalent.
* `c_src/CMakeLists.txt`: no `option()`, no compile definitions, no `#ifdef` in
  `src/lib.c` / `include/lib.h` → exactly one C configuration.

So every row below is verified under the single (and therefore *every*) feature
combination.

## Runtime configuration axes (derived from the `if` / `switch` / arithmetic branches in `c_src/src/lib.c`)

| axis | values the C code actually distinguishes |
|------|------------------------------------------|
| `create_state.capacity` | `< 0` (malloc fails) · `0` (`malloc(0)`, `snprintf(…,0,…)` writes nothing) · `1 … 24` (`snprintf` truncates) · exactly-fits · `> needed` · `INT_MAX` |
| `create_state.initial_val` | `0` · positive · negative · `INT_MIN` / `INT_MAX` (widest `%d` renderings) — also becomes the union's raw bit pattern |
| `PackedFlags` state | `flag1/2/3` (1 bit each) · `counter` (5 bits, wraps mod 32) · `mode` (3 bits) · `status` (5 bits, only ever set to 15) · `reserved` (16 bits) |
| `update_flags.param` | low 3 bits → `flag1/flag2/flag3` · bits 3-5 → `mode` (8 values) · sign (arithmetic `>>`) · repeat count → `counter` wrap 0→31→0 |
| `process_buffer.target` | absent · present once · present many/consecutive · at offset 0 · at last index · `'\0'` · high-bit byte `0x80-0xFF` (negative `char`) |
| `process_buffer` buffer shape | empty (`strlen == 0`) · 1 byte · long · all-same-byte · random bytes |
| `confuse_types.operation` | `0` (write int) · `1` (read float + `(int)` cast) · `2` (read uint, `& 0xFF`) · `3` (read 4 signed bytes) · out of range |
| `confuse_types` union payload | arbitrary 32-bit pattern → `int` / `float` (normal, denormal, ±0, ±Inf, NaN, out-of-`int`-range) / `unsigned` / 4 signed bytes |
| `confusion.param3` | `% 10` ∈ `{-9…9}` → search byte `'0' + n` (below `'0'` when negative) |
| `confusion.param4` | `% 4` ∈ `{-3…3}` → `case 0..3` or no case |

## Configuration rows

Every row is exercised with **many randomised inputs** (fixed seed
`0x5EED_C0FFEE`, splitmix64 PRNG) unless the row is a single exact boundary
value; both the C `.so` and the Rust `.so` are called through `dlopen`/`dlsym`
and the returned value **and the byte-exact stdout** are compared.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `create_state` + `destroy_state` | `capacity = 128`, `initial_val` random over the full `i32` range | `cfg_01_create_state_random_initial_val` | [x] |
| 2 | `create_state` | `capacity = 128`, `initial_val ∈ {0, 1, -1, INT_MIN, INT_MAX, 1078530011}` (boundary renderings) | `cfg_02_create_state_boundary_initial_val` | [x] |
| 3 | `create_state` | `capacity ∈ 0..=40` × random `initial_val` (covers `malloc(0)`, every `snprintf` truncation point, exact fit, slack) | `cfg_03_create_state_capacity_sweep` | [x] |
| 4 | `create_state` | `capacity` large (`4096`, `65536`, `1 << 20`) | `cfg_04_create_state_large_capacity` | [x] |
| 5 | `create_state` | initial `PackedFlags` bit pattern: `flag1=1 flag2=0 flag3=1 counter=0 mode=3 status=15 reserved=0` → raw storage unit must be identical | `cfg_05_create_state_initial_flag_bits` | [x] |
| 6 | `process_buffer` | default `create_state` buffer, `target` = every byte `0x00..=0xFF` | `cfg_06_process_buffer_all_targets` | [x] |
| 7 | `process_buffer` | hand-written buffer, `target` present exactly once at index 0 | `cfg_07_process_buffer_match_first` | [x] |
| 8 | `process_buffer` | hand-written buffer, `target` present exactly once at the last index | `cfg_08_process_buffer_match_last` | [x] |
| 9 | `process_buffer` | hand-written buffer of *consecutive* matches (`"aaaa…"`, tests the `remaining -= found - ptr + 1` update) | `cfg_09_process_buffer_consecutive` | [x] |
| 10 | `process_buffer` | random buffers (random length 0..=120, random non-NUL bytes) × random `target` | `cfg_10_process_buffer_random` | [x] |
| 11 | `process_buffer` | buffer with an embedded NUL well before `capacity` (so `strlen` < capacity) and matches on both sides of it | `cfg_11_process_buffer_embedded_nul` | [x] |
| 12 | `process_buffer` | buffer bytes with the high bit set (negative `char`) × high-bit `target` | `cfg_12_process_buffer_high_bit_bytes` | [x] |
| 13 | `update_flags` | `param = 0..=63` (full cross-product of `flag1`,`flag2`,`flag3` × `mode` 0..7), single call | `cfg_13_update_flags_low_six_bits` | [x] |
| 14 | `update_flags` | called 40× in a row on the same state → `counter` 1,2,…,31,0,1,… wrap-around | `cfg_14_update_flags_counter_wrap` | [x] |
| 15 | `update_flags` | random full-range `i32` `param` (negative → arithmetic `>>3`) | `cfg_15_update_flags_random_param` | [x] |
| 16 | `update_flags` | verifies `status` (15) and `reserved` (0) are *preserved* — raw 32-bit `PackedFlags` word compared after each call | `cfg_16_update_flags_preserves_status_reserved` | [x] |
| 17 | `confuse_types` | `operation = 0`, arbitrary prior payload → writes `1078530011`, prints, returns `0`; union word compared afterwards | `cfg_17_confuse_types_op0` | [x] |
| 18 | `confuse_types` | `operation = 1`, payload = random full-range `u32` bit patterns (normals, denormals, ±0, ±Inf, NaN) | `cfg_18_confuse_types_op1_random` | [x] |
| 19 | `confuse_types` | `operation = 1`, payload = curated float boundaries: `0.0`, `-0.0`, `±Inf`, quiet & signalling NaN, `FLT_MIN`, denormal, `21474836.0` (`*100` ≈ `INT_MAX`), `21474837.0`, `-21474836.0`, `1078530011` (π) | `cfg_19_confuse_types_op1_boundaries` | [x] |
| 20 | `confuse_types` | `operation = 2`, payload = random `u32` incl. `0`, `0xFF`, `0x100`, `0xFFFFFFFF` (`& 0xFF` masking + `%u` printing) | `cfg_20_confuse_types_op2` | [x] |
| 21 | `confuse_types` | `operation = 3`, payload = random `u32` incl. bytes ≥ `0x80` (signed-`char` promotion in `%d` and in `bytes[0] + bytes[1]`) | `cfg_21_confuse_types_op3` | [x] |
| 22 | `confuse_types` | full sweep: `operation ∈ {0,1,2,3}` × 8 curated payloads, run in sequence on one state so `op 0`'s write is observed by later ops | `cfg_22_confuse_types_sequenced` | [x] |
| 23 | `confusion` | random full-range `(param1, param2, param3, param4)` — the composed pipeline | `cfg_23_confusion_random` | [x] |
| 24 | `confusion` | `param4 % 4` forced to each of `0,1,2,3` × random other params | `cfg_24_confusion_each_operation` | [x] |
| 25 | `confusion` | `param3 % 10` forced to each of `0..9` (search byte `'0'..'9'`, i.e. the digits that actually occur in `"State:%d:Mode:%d"`) | `cfg_25_confusion_each_search_digit` | [x] |
| 26 | `confusion` | `param3` negative → search byte `<'0'` (`'('`…`'/'`, incl. `':'`-adjacent bytes) | `cfg_26_confusion_negative_param3` | [x] |
| 27 | `confusion` | `param2` sweep `0..=63` (all `flag`/`mode` combinations affect both stdout and the `mode * 3` term) | `cfg_27_confusion_param2_sweep` | [x] |
| 28 | `confusion` | `param1` chosen so `confuse_types(op 1)` sees NaN/Inf/huge (`(int)` cast → `INT_MIN`) and the final `result +=` chain wraps around | `cfg_28_confusion_int_min_wrap` | [x] |
| 29 | `confusion` | all 4 params at `INT_MIN` / `INT_MAX` / `0` / `-1` cross-product (81 combinations) | `cfg_29_confusion_extremes_cross` | [x] |
| 30 | low-level pipeline, hand-driven | `create_state` → `update_flags` ×k → `process_buffer` → `confuse_types` → read `flags`/`data`/`capacity` → `destroy_state`, with randomised `capacity`, `k`, params — i.e. the same composition `confusion` performs but through the low-level exports with non-default `capacity` | `cfg_30_low_level_pipeline_random` | [x] |
