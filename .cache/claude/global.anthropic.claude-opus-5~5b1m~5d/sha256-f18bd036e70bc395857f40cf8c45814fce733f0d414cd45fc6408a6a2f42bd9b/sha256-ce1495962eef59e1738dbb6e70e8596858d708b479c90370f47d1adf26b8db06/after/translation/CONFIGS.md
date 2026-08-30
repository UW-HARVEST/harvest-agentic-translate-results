# CONFIGS.md — configuration-surface table (valid inputs)

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Axes the C code actually branches on

| axis | values the C distinguishes | evidence |
|------|---------------------------|----------|
| A. entry point | `driver`, `good`, `bad`, `printIntLine`, `printLine` | the 5 `T` symbols in `nm -D`; only `driver` is in the public header, the other 4 are the *lower-level* entry points and are exported, so a real consumer can `dlsym` them |
| B. `driver` mode flag | `useGood == 0` → `bad()`; `useGood != 0` → `good()` | `if (useGood)` at driver.c:75 |
| C. `printLine` pointer state | `NULL` vs non-`NULL` | `if(line != NULL)` at driver.c:32 |
| D. `printLine` payload shape | empty / 1 byte / short / exactly at & around stdio buffer boundaries (`BUFSIZ`-1,`BUFSIZ`,`BUFSIZ`+1, 4095/4096/4097) / huge (1 MiB); ASCII / control bytes / `%` conversion specifiers / non-UTF-8 0x80..0xFF | `printf("%s\n", …)`, driver.c:34 — payload is data, and stdio buffering is the only size-dependent behaviour |
| E. `printIntLine` value shape | `0`, `1`, `-1`, single/multi digit, `INT_MIN`, `INT_MAX`, ±powers of two, randomized full-range | `printf("%d\n", …)`, driver.c:40 |
| F. call multiplicity / interleaving | one / many / alternating `good`+`bad` / `printLine`+`printIntLine` interleaved in one capture | stdio is a shared global stream, so ordering & buffering are observable across calls |
| G. build configuration | Cargo.toml declares **no `[features]`**, so the only combos are default / `--no-default-features`; profiles `dev` (unwind) and `release` (`panic = "abort"`) | `translation/Cargo.toml` |

Everything is compared as **byte-exact `stdout`** (all five functions return `void`),
captured by redirecting fd 1 around each call and `fflush`ing, so buffering
differences would show up too.

Randomization: `SEED = 0x5EED_1234_5EED_1234`, a deterministic xorshift64\* PRNG in
the test file, ≥256 randomized inputs per randomized row.

## Rows

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `printIntLine` | `intNumber = 0` (the value the rest of the library actually prints) | `cfg_c1_print_int_zero` | [x] |
| C2 | `printIntLine` | hand-picked small magnitudes: `1, -1, 9, 10, -9, -10, 99, 100, -100` (digit-count transitions) | `cfg_c2_print_int_small` | [x] |
| C3 | `printIntLine` | boundaries: `INT_MAX`, `INT_MAX-1`, `INT_MIN`, `INT_MIN+1`, `-2147483647` | `cfg_c3_print_int_boundaries` | [x] |
| C4 | `printIntLine` | ± powers of two and their neighbours (`±2^k`, `±2^k±1`, k=0..31) | `cfg_c4_print_int_powers_of_two` | [x] |
| C5 | `printIntLine` | 4096 randomized full-range `i32` values (seeded) | `cfg_c5_print_int_random` | [x] |
| C6 | `printIntLine` | 512 randomized values issued back-to-back inside ONE capture (buffering/order) | `cfg_c6_print_int_batched` | [x] |
| C7 | `printLine` | non-NULL, empty payload `""` | `cfg_c7_print_line_empty` | [x] |
| C8 | `printLine` | non-NULL, 1-byte payloads: every byte value `0x01..0xFF` (no NUL) | `cfg_c8_print_line_every_single_byte` | [x] |
| C9 | `printLine` | plain ASCII words / sentences with spaces | `cfg_c9_print_line_ascii` | [x] |
| C10 | `printLine` | payload containing `%` conversion specifiers (`%s %d %n %% %.9999f`) | `cfg_c10_print_line_percent_payload` | [x] |
| C11 | `printLine` | payload of control bytes (`\n \t \r \x0b \x1b \x07`) — embedded newlines | `cfg_c11_print_line_control_bytes` | [x] |
| C12 | `printLine` | payload of non-UTF-8 / high bytes `0x80..0xFF` (invalid UTF-8 sequences, lone continuation bytes, overlong forms) | `cfg_c12_print_line_non_utf8` | [x] |
| C13 | `printLine` | valid multi-byte UTF-8 (2/3/4-byte sequences, emoji, combining marks) | `cfg_c13_print_line_utf8` | [x] |
| C14 | `printLine` | length sweep across stdio buffer boundaries: 1,2,63,64,65,127,128,129,255,256,257,511,512,513,1023,1024,1025,4095,4096,4097,8191,8192,8193,`BUFSIZ`±1 | `cfg_c14_print_line_length_sweep` | [x] |
| C15 | `printLine` | huge payloads: 64 KiB, 256 KiB, 1 MiB of random bytes | `cfg_c15_print_line_huge` | [x] |
| C16 | `printLine` | 512 randomized payloads: random length 0..300, random bytes `0x01..0xFF` (seeded) | `cfg_c16_print_line_random` | [x] |
| C17 | `printLine` | 256 randomized payloads issued back-to-back inside ONE capture | `cfg_c17_print_line_batched` | [x] |
| C18 | `good` | called directly (low-level entry point), single call | `cfg_c18_good_direct` | [x] |
| C19 | `bad` | called directly (low-level entry point), single call — the `alloca(10)` path | `cfg_c19_bad_direct` | [x] |
| C20 | `good`, `bad` | 64 alternating calls in ONE capture (frame reuse after the OOB write) | `cfg_c20_good_bad_alternating` | [x] |
| C21 | `driver` | `useGood = 1` → `good()` | `cfg_c21_driver_true` | [x] |
| C22 | `driver` | `useGood = 0` → `bad()` | `cfg_c22_driver_false` | [x] |
| C23 | `driver` | 1024 randomized `i32` flags (seeded) — mostly non-zero, `0` mixed in | `cfg_c23_driver_random_flag` | [x] |
| C24 | `driver` | randomized sequence of flags in ONE capture (mode switching mid-stream) | `cfg_c24_driver_random_sequence` | [x] |
| C25 | mixed: `printLine` + `printIntLine` + `driver` + `good` + `bad` | randomized interleaving of all five entry points in ONE capture — the composed pipeline, ordering & buffering across function boundaries | `cfg_c25_all_entry_points_interleaved` | [x] |
| C26 | all five | repeat the whole suite under `--no-default-features` (only other feature combo; Cargo.toml has no `[features]`) | `run_all_feature_combos.sh` | [x] |
