# CONFIGS.md — configuration-surface table (valid inputs)

Axes mechanically taken from the branches of `c_src/src/lib.c` and
`c_src/src/main.c`:

* **entry points** — `process_strings` is the only exported symbol; through the
  `switch` it reaches the five `static` workers (`validate_token`,
  `parse_command`, `compare_prefix`, `find_delimiter`, `match_pattern`). The
  driver `main` is the second entry point (stdin token stream).
* **runtime options** — `operation` (`0,1,2,3,4`, default), `flags & 0x01`
  (`exact` in `compare_prefix`), `flags & 0x02` (`case_sensitive` in
  `match_pattern`); all other `flags` bits are ignored and must stay ignored.
* **input shapes** — NUL terminated vs not (the whole point of the code),
  `input_len`/`ref_len` relative to the literal lengths
  (`STOP`=4, `START`/`PAUSE`/`RESET`=5, `RESUME`=6, `ADMIN`=5, `VALID`=5, `OK`=2),
  `len == 0`, `len == 1024` (`MAX_BUFFER_SIZE`), string lengths around the
  fixed 64-byte scratch buffers (`expected[64]`, `wildcard_patterns[3][64]` →
  truncation at 63 chars), `strlen(text)` vs `strlen(pattern)`
  (`<`, `==`, `>`), delimiter `':'` / `'|'` / other / inside vs outside the
  first `len` bytes, embedded NULs, bytes ≥ 0x80 (signed `char` comparisons).

Every row is exercised through **both** `.so` exports with many randomised
inputs (fixed seed) in `tests/ffi_diff.rs`, and additionally at the executable
level in `tests/exe_diff.rs`.

| #  | entry point(s) | configuration (options + input shape) | test | [x] |
|----|----------------|----------------------------------------|------|-----|
| 1  | `process_strings` → `validate_token` | op 0, input NUL terminated, reference NUL terminated, random contents | `cfg_op0_terminated` | [x] |
| 2  | `process_strings` → `validate_token` | op 0, input == reference (exact match → 1), all lengths 0…16 | `cfg_op0_equal_strings` | [x] |
| 3  | `process_strings` → `validate_token` | op 0, input `"VALID"` / `"OK"` special-cased second/third `strcmp` | `cfg_op0_valid_ok_literals` | [x] |
| 4  | `process_strings` → `validate_token` | op 0, input **not** NUL terminated (overread of `input`) | `cfg_op0_unterminated_input` | [x] |
| 5  | `process_strings` → `validate_token` | op 0, reference **not** NUL terminated (overread of `reference`) | `cfg_op0_unterminated_ref` | [x] |
| 6  | `process_strings` → `parse_command` | op 1, `input_len` ≥ `cmd_len` and `buffer[cmd_len]` is `'\0'` → index 0…4 | `cfg_op1_exact_commands` | [x] |
| 7  | `process_strings` → `parse_command` | op 1, `buffer[cmd_len] == ' '` (space-terminated command) | `cfg_op1_space_terminated` | [x] |
| 8  | `process_strings` → `parse_command` | op 1, `input_len < cmd_len` (skips the `strncmp` branch, falls through to `strcmp`) | `cfg_op1_short_buf_size` | [x] |
| 9  | `process_strings` → `parse_command` | op 1, `input_len` = 0 / 1 / 4 / 5 / 6 boundaries around the literals | `cfg_op1_len_boundaries` | [x] |
| 10 | `process_strings` → `parse_command` | op 1, `"ADMIN"` → 99 | `cfg_op1_admin` | [x] |
| 11 | `process_strings` → `parse_command` | op 1, `input_len` inconsistent with the data (larger/smaller than the real string) | `cfg_op1_len_vs_data` | [x] |
| 12 | `process_strings` → `compare_prefix` | op 2, `flags&1 == 0` (loose), prefix shorter than str | `cfg_op2_loose_prefix` | [x] |
| 13 | `process_strings` → `compare_prefix` | op 2, `flags&1 == 0`, `strlen(prefix) == 0` (`strncmp(_,_,0)` → always 1) | `cfg_op2_loose_empty_prefix` | [x] |
| 14 | `process_strings` → `compare_prefix` | op 2, `flags&1 == 1` (exact), equal strings → 1 | `cfg_op2_exact_equal` | [x] |
| 15 | `process_strings` → `compare_prefix` | op 2, exact, str == prefix + `_v1`/`_v2`/`_old`/`_new`/`_tmp` → 2…6 | `cfg_op2_exact_variations` | [x] |
| 16 | `process_strings` → `compare_prefix` | op 2, exact, prefix long enough that `strncpy(expected,prefix,63)`+`strncat` truncates (prefix len 55…70) | `cfg_op2_exact_truncation` | [x] |
| 17 | `process_strings` → `compare_prefix` | op 2, both flags bits set (`flags&2` must be ignored here) | `cfg_op2_flag_bit1_ignored` | [x] |
| 18 | `process_strings` → `compare_prefix` | op 2, unterminated prefix and/or str (overread) | `cfg_op2_unterminated` | [x] |
| 19 | `process_strings` → `find_delimiter` | op 3, `ref_len > 0` → delimiter `reference[0]`, delimiter present inside `len` | `cfg_op3_delim_found` | [x] |
| 20 | `process_strings` → `find_delimiter` | op 3, `ref_len == 0` → delimiter defaults to `':'` | `cfg_op3_default_colon` | [x] |
| 21 | `process_strings` → `find_delimiter` | op 3, embedded NUL before the delimiter (`break`, then the `strcmp` specials) | `cfg_op3_nul_before_delim` | [x] |
| 22 | `process_strings` → `find_delimiter` | op 3, delimiter `'|'` + data `"NONE"` → -2; delimiter `':'` + data `"EMPTY"` → -3 | `cfg_op3_special_patterns` | [x] |
| 23 | `process_strings` → `find_delimiter` | op 3, delimiter is `'\0'` (`reference[0] == 0`) — matches the terminator | `cfg_op3_nul_delimiter` | [x] |
| 24 | `process_strings` → `find_delimiter` | op 3, `len` 1 / 1023 / 1024, delimiter at first/last position and absent | `cfg_op3_len_boundaries` | [x] |
| 25 | `process_strings` → `find_delimiter` | op 3, delimiter byte ≥ 0x80 (signed `char` comparison) | `cfg_op3_high_bit_delim` | [x] |
| 26 | `process_strings` → `match_pattern` | op 4, `flags&2 != 0`, exact match → 1 | `cfg_op4_cs_exact` | [x] |
| 27 | `process_strings` → `match_pattern` | op 4, cs, text == `"*p*"` / `"p*"` / `"*p"` → 2/3/4 | `cfg_op4_cs_wildcards` | [x] |
| 28 | `process_strings` → `match_pattern` | op 4, cs, pattern occurs inside text at position i → 10+i (i = 0 … text_len-pattern_len) | `cfg_op4_cs_substring` | [x] |
| 29 | `process_strings` → `match_pattern` | op 4, cs, `strlen(pattern) == 0` (empty pattern → 10 + 0 via `strncmp(_,_,0)`) | `cfg_op4_cs_empty_pattern` | [x] |
| 30 | `process_strings` → `match_pattern` | op 4, cs, pattern ≥ 62 chars so `snprintf(..., 64, "*%s*")` truncates | `cfg_op4_cs_wildcard_truncation` | [x] |
| 31 | `process_strings` → `match_pattern` | op 4, `flags&2 == 0`, exact match → 1 | `cfg_op4_ci_exact` | [x] |
| 32 | `process_strings` → `match_pattern` | op 4, ci, `text_len != pattern_len` and prefix match → 5 | `cfg_op4_ci_prefix` | [x] |
| 33 | `process_strings` → `match_pattern` | op 4, ci, equal length, case-insensitive match → 6 (mixed case, `A`-`Z`/`a`-`z` boundaries `@`,`[`,`` ` ``,`{`) | `cfg_op4_ci_equal_len` | [x] |
| 34 | `process_strings` → `match_pattern` | op 4, ci, equal length, differing → 0 | `cfg_op4_ci_no_match` | [x] |
| 35 | `process_strings` → `match_pattern` | op 4, unterminated text and/or pattern (overread) with `flags&2` both ways | `cfg_op4_unterminated` | [x] |
| 36 | `process_strings` | all ops × `flags` = 0/1/2/3/0xFFFFFFFF (only bits 0 and 1 may matter, and only for ops 2 and 4) | `cfg_flags_cross_product` | [x] |
| 37 | `process_strings` | all ops × `input_len`/`ref_len` = 0 (empty buffers, still non-NULL) | `cfg_zero_lengths_all_ops` | [x] |
| 38 | `process_strings` | all ops × random binary buffers (bytes 0x00…0xFF incl. ≥ 0x80), lengths 0…40 | `cfg_random_binary_all_ops` | [x] |
| 39 | `process_strings` | all ops × long buffers (len 1023/1024) | `cfg_long_buffers_all_ops` | [x] |
| 40 | `main` (driver) | full stdin pipeline: `operation flags input_len bytes… ref_len bytes…`, all ops × flags, NUL terminated buffers, lengths 0…1024 | `exe_diff.rs::exe_random_all_ops`, `exe_boundary_lengths`, `exe_literal_results`, `exe_accepts_maximum_lengths` | [x] |
| 41 | `main` (driver) | scanf tokenisation shapes: multiple spaces/newlines/tabs between tokens, `+`/`-` signs, `0x` prefixes, values overflowing `int`/`unsigned`/`size_t`, trailing/missing tokens | `exe_diff.rs::exe_scanf_shapes`, `exe_scanf_shapes_with_overread` | [x] |
| 42 | `main` (driver) | unterminated buffers of every length 0…100 (the reads run off the data into the uninitialised frame), all ops × flags | `exe_frame.rs::exe_frame_unterminated_lengths` | [x] |
| 43 | `main` (driver) | full 1024 byte unterminated buffers: the reads cross from `input_buffer` into the locals of `main`, and from `ref_buffer` into `input_buffer` | `exe_frame.rs::exe_frame_full_buffers` | [x] |
| 44 | `main` (driver) | randomised pipeline including unterminated buffers, literals, binary data and long buffers | `exe_frame.rs::exe_frame_random` | [x] |
| 45 | `main` (driver) | `match_pattern` loop-bound underflow: the unbounded scan walks off the stack (SIGSEGV) or finds a one byte pattern in the frame junk at a specific offset | `exe_frame.rs::exe_frame_unbounded_loop`, `exe_frame_unbounded_loop_finds_junk_byte` | [x] |
| 46 | `main` (driver) | the modelled frame snapshot itself vs a fresh ptrace capture of the real frame | `exe_frame.rs::exe_frame_snapshot_matches_reality` | [x] |

## Notes on the uninitialised stack frame

The C code calls `strcmp`/`strlen` on buffers that need not be NUL terminated,
so for a large part of its input space its result depends on the *uninitialised*
bytes of `main`'s stack frame. `src/mem.rs` models that frame exactly as GCC lays
it out and `src/frame_junk.rs` holds a byte-exact snapshot of the left-overs,
captured from the compiled C program with `probe/dump_frame.c` (ptrace, 24 runs,
majority vote per byte).

Two properties of those bytes matter:

* the **zero pattern** (which regions are zero filled) is what `strlen`/`strcmp`
  branch on, and it is structural: it is identical across runs, environments and
  builds. `exe_frame_snapshot_matches_reality` asserts that every byte the
  snapshot models as zero really is zero.
* the **values** of the non-zero bytes are the loader's saved stack pointers.
  Bytes 1-3 of such a pointer change with ASLR on every run, and byte 0 changes
  with the size of the environment plus the length of the path the program was
  exec'd with (it is `addr & 0xff` of a 16 byte aligned address, so it is zero for
  one environment out of sixteen). No static table - and no translation - can
  reproduce those for every environment, and the C program itself is not
  reproducible there either. Measured noise floor: 5 of 2046 probed offsets in
  one environment, 22 of 2096 in another (~1%).

Because of that the executable level tests are split: `exe_diff.rs` compares the
two real binaries on inputs that never read uninitialised memory, and
`exe_frame.rs` compares them with `probe/inject_frame.c` forcing the C program's
uninitialised frame bytes to the modelled snapshot - which makes the overread
paths fully deterministic and environment independent.

### The C program is not deterministic there

Example (`probe/fuzz_diff2.py` found it): `operation 4`, `flags 2`,
`input_len 10` = `233 131 112 105 213 70 174 208 123 0`, `ref_len 0`. The pattern
is `ref_buffer` itself, so `strlen(pattern)` is the length of the loader's
left-over stack pointer at `ref_buffer[0]`, whose low byte is `addr & 0xff` of a
16 byte aligned address:

```
$ for i in $(seq 1 60); do echo "4 2 10 233 131 112 105 213 70 174 208 123 0 0" | c_src/build/driver; done | sort | uniq -c
     54 0        <- ref_buffer[0] != 0  -> strlen(pattern) == 6
      6 10       <- ref_buffer[0] == 0  -> strlen(pattern) == 0, match at offset 0
$ ... | target/release/driver | sort | uniq -c
     60 0
```

The snapshot models that byte as non-zero, which is what the C program does in
15 of 16 runs, so the translation reproduces the majority behaviour. No table (and
no translation) can do better than that, because the C program answers
differently from one run to the next.

`probe/fuzz_stable.py` therefore runs the C program five times per input and only
reports a divergence when the C result is stable. Measured over the randomised
corpora: **0 stable divergences**; the only differences are inputs on which the C
program contradicts itself between runs (~0.3% of random inputs).
