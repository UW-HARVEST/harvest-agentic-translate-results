# ERRORS.md — error-surface table (Phase A / Phase C)

Derived mechanically from `c_src/src/lib.c` by grepping **every** rejection site:

```sh
grep -n 'return 0\|return NULL\|assert(\|cp_error_reason =\|goto cp_err' c_src/src/lib.c
```

The C library is compiled **without** `NDEBUG` (`CMakeLists.txt` sets no
`CMAKE_BUILD_TYPE` and no `-DNDEBUG`), so every `assert()` is live and a failure
calls `__assert_fail` → `abort()` → **SIGABRT**.  `src/lib.rs` mirrors this with
`cp_assert!` → `std::process::abort()` → SIGABRT.

`tests/diff.rs` runs every case in a forked grandchild, so per case it compares

* the value returned by `cp_inflate`,
* the whole output arena,
* the NUL-terminated `cp_error_reason` string (`check_group_expect_error`
  additionally pins the exact expected literal, so "both failed somehow" is not
  accepted), and
* the child's termination: exit code, or fatal signal, or SIGALRM when the C
  library does not terminate at all.

For the assertion rows the test also greps the C child's **stderr** for the text
of the assertion that is supposed to have fired, which is how each row below is
confirmed to exercise the intended `assert()` rather than an earlier one.
(The assertion *message* itself is not reproduced by the Rust build: it contains
glibc's `<progname>: <__FILE__>:<line>: <func>:` prefix, which is a property of
the C build tree and not a value returned across the FFI boundary.  The signal,
the return value and every byte of state are reproduced.)

Legend for "expected C result":
* `ret 0` – `cp_inflate` returns `0` and `cp_error_reason` points at the quoted
  string.
* `SIGABRT` – the process aborts inside `assert()`.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test group | ✔ |
|----|----------|---------------------------------------------|-------------------|------------|---|
| 1  | `cp_stored` (lib.c:170) | `LEN != (uint16_t)~NLEN` in a `btype==0` block | `ret 0`, `"Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` | `err_len_nlen_mismatch` (5 cases) | [x] |
| 2  | `cp_stored` (lib.c:179) | `!(s->bits_left / 8 <= (int)LEN)` – the stored block is shorter than the input that is left (trailing bytes after the payload) | `ret 0`, `"Stored block extends beyond end of input stream."` | `err_stored_beyond_end` (5 cases) | [x] |
| 3  | `cp_block` (lib.c:254) | literal symbol decoded but `!(s->out + 1 <= s->out_end)` – `out` full / `out_bytes == 0` / `out_bytes < 0` | `ret 0`, `"Attempted to overwrite out buffer while outputting a symbol."` | `err_out_full_symbol` (4), `err_out_bytes_zero` (1), `err_out_bytes_negative` (3, incl. `INT_MIN`) | [x] |
| 4  | `cp_block` (lib.c:273) | back-reference with `!(s->out - backwards_distance >= s->begin)` | `ret 0`, `"Attempted to write before out buffer (invalid backwards distance)."` | `err_backdist_before_begin` (5) | [x] |
| 5  | `cp_block` (lib.c:282) | back-reference with `!(s->out + length <= s->out_end)` | `ret 0`, `"Attempted to overwrite out buffer while outputting a string."` | `err_match_past_out_end` (4) | [x] |
| 6  | `cp_inflate` (lib.c:356) | `btype == 3` (reserved), as the first *and* as a later block, every input alignment | `ret 0`, `"Detected unknown block type within input stream."` | `err_btype3` (9) | [x] |
| 7  | `cp_unfilter` (lib.c:434) | first row's PNG filter byte `> 4` | `return 0` | *unobservable* — `cp_unfilter` is `static` and **called by nothing** in this translation unit, so it has internal linkage and is absent from the dynamic symbol table of **both** `.so` files. `err_dead_static_not_exported` asserts that (19 names). The Rust translation contains the same logic, verified by inspection. | [x] |
| 8  | `cp_unfilter` (lib.c:468) | a non-first row's filter byte `> 4` | `return 0` | *unobservable*, as #7 | [x] |
| 9  | `cp_chunk` (lib.c:397) | `memcmp(start+4, chunk, 4) != 0` **or** `len < minlen` **or** `png->p + len + 12 > png->end` | `return NULL` | *unobservable*, as #7 (`cp_chunk` is `static` and uncalled) | [x] |
| 10 | `cp_find` (lib.c:409) | no chunk with the requested id/`minlen` before `png->end` | `return NULL` | *unobservable*, as #7 (`cp_find` is `static` and uncalled) | [x] |
| 11 | `cp_ptr` (lib.c:89) | `s->bits_left & 7` — a `btype==0` block reached after the *partial* final word was folded in at a `count % 8 != 0` boundary. Input `62 60 00 E4 FF 1F 00`: fixed block with two 8-bit literals, the final word folds in at `count==13`, then a stored block with `LEN=0xFFFF`, `NLEN=0` passes both checks and `bits_left == -5`. | `SIGABRT` (`!(s->bits_left & 7)`) | `abort_cp_ptr_misaligned` | [x] |
| 12 | `cp_peak_bits` (lib.c:98) | `s->word_index > s->word_count` after the post-increment | `SIGABRT` | *unreachable by construction*: the increment is guarded by `if (s->word_index < s->word_count)`, so the post-increment value is `<= word_count`. Present and identical in Rust; never tripped by any of the 8015 cases in either library. | [x] |
| 13 | `cp_consume_bits` (lib.c:109) | `s->count < num_bits_to_read` — truncated stream where `bits_left + count >= n` (so the overflow assert passes) but fewer than `n` bits are buffered | `SIGABRT` (`s->count >= num_bits_to_read`) | `abort_stream_exhausted` (54 truncation lengths, 13 of which abort) | [x] |
| 14 | `cp_read_bits` (lib.c:117) | `num_bits_to_read > 32` — reachable through the writable exported tables `cp_len_extra_bits` / `cp_dist_extra_bits` (`uint8_t`, so up to 255) | `SIGABRT` (`num_bits_to_read <= 32`) | `abort_read_bits_gt32_len` (33), `abort_read_bits_gt32_len_255` (255), `abort_read_bits_gt32_dist` (40) | [x] |
| 15 | `cp_read_bits` (lib.c:118) | `num_bits_to_read < 0` | `SIGABRT` | *unreachable by construction*: every call site passes a literal (`1,2,3,4,5,7,16`), `s->count & 7` (`0..7`; `count >= 0` is enforced by #13), or a `uint8_t` table element (`0..255`). Present and identical in Rust. | [x] |
| 16 | `cp_read_bits` (lib.c:119) | `s->bits_left <= 0` — `in_bytes == 0`, `in_bytes < 0`, `in_bytes*8` overflowing to 0, or the stream running dry | `SIGABRT` (`s->bits_left > 0`) | `abort_null_in_zero_len`, `abort_in_bytes_zero`, `abort_in_bytes_negative_{1,2,3,4}`, `abort_in_bytes_min`, `abort_in_bytes_overflow`, `abort_in_bytes_extremes` (28), `abort_stream_exhausted` | [x] |
| 17 | `cp_read_bits` (lib.c:120) | `s->count > 64` | `SIGABRT` | *unreachable by construction*: `count` only grows in `cp_peak_bits`, guarded by `count < num_bits_to_read`; `num_bits_to_read <= 32` is asserted (#14) before every `cp_peak_bits` reached from `cp_read_bits`, and `cp_decode` peaks with a literal 16, so `count <= 15 + 32 = 47`. Present and identical in Rust; never tripped. | [x] |
| 18 | `cp_read_bits` (lib.c:121) | `cp_would_overflow(s, n)`, i.e. `(bits_left + count) - n < 0`. Input `62 98 00 04 80 00 00 00 00` (9 bytes, aligned): both words fold in (`F = 64`) and exactly 64 bits are consumed by the time `cp_stored` does its **second** 16-bit read, leaving `bits_left = 8`, `count = 0` → `8 - 16 = -8`. | `SIGABRT` (`!cp_would_overflow(...)`) | `abort_would_overflow` (stderr checked for `cp_would_overflow`) | [x] |
| 19 | `cp_build` (lib.c:148) | a code length `>= 16`, reachable through the writable exported `cp_fixed_table` | `SIGABRT` (`len < 16`) | `abort_build_len_ge16_{16,17,20,31,32,40,47,63,100,255}` | [x] |
| 20 | `cp_decode` (lib.c:211) | `(search >> len) != (key >> len)` — the buffered bits match no code of the tree. Two shapes: an incomplete code-length tree (`abort_decode_no_match`) and an **empty** tree (`hi == 0` → `tree[-1]`, `len == 32`, and `search >> 32` degenerates to a shift by 0 on x86-64, so the comparison is `search == 0`, which is impossible because `search >= 0xFFFF`). | `SIGABRT` (`(search >> len) == (key >> len)`) | `abort_decode_no_match`, `abort_dynamic_empty_cl_tree` (`HCLEN == 4`) | [x] |

## Generic FFI boundary conditions (also covered)

| #  | case | notes | test group | ✔ |
|----|------|-------|------------|---|
| G1 | `cp_inflate(NULL, 0, NULL, 0)` | `in_bytes*8 == 0` → assert #16 fires before either pointer is dereferenced | `abort_null_in_zero_len` | [x] |
| G2 | `cp_inflate(in, 0, out, n)` | `bits_left == 0` → assert #16 | `abort_in_bytes_zero` | [x] |
| G3 | `cp_inflate(in, n, out, 0)` | `out_end == out` → error #3 on the first literal | `err_out_bytes_zero` | [x] |
| G4 | `cp_inflate(in, n, out, <0>)` | `out_end < out` → error #3; `-1`, `-1000`, `INT_MIN` | `err_out_bytes_negative` | [x] |
| G5 | `cp_inflate(in, <0>, …)` | `bits_left = in_bytes*8 < 0` → assert #16 for `-1..-4`; for `-12345` the final-word fold-in loop indexes `in[in_bytes - last_bytes + i]` = `in[-12348..]`, so **both** libraries walk off the front of the arena and die from the same signal (SIGSEGV) | `abort_in_bytes_negative_*` | [x] |
| G6 | `in_bytes` where `in_bytes*8` overflows | `0x2000_0000`, `0x4000_0000`, `0x6000_0000` wrap to `bits_left == 0` → assert #16; `INT_MIN` crossed with all four alignments, where `in_bytes - first_bytes` also underflows. The Rust translation uses `wrapping_*` throughout so it wraps like C instead of trapping (a debug-profile `-` would abort with a different message). | `abort_in_bytes_overflow`, `abort_in_bytes_extremes`, `abort_in_bytes_min_align{0,1,2,3}` | [x] |
| G7 | `convert_pix` `bpp` outside the `switch` — a C `switch` on `int` accepts **any** value, i.e. the "out-of-range enum across FFI" case | `0, 5, 6, 255, -1, -4, INT_MIN, INT_MAX`: no store to `dst`, but `src++` still runs `h` times and `src += bpp` `w*h` times | `cfg06_convert_pix_bpp_out_of_range` | [x] |
| G8 | `convert_pix` with `w <= 0` and/or `h <= 0`, incl. `INT_MIN` / `INT_MAX` | the loops do not execute; `dst` untouched | `cfg07_convert_pix_degenerate` | [x] |
| G9 | `convert_pix(bpp, w, h, NULL, NULL)` with `h <= 0`, and with `w <= 0` (pointer arithmetic only) | no dereference of the NULL pointers | `cfg07_convert_pix_degenerate` | [x] |
| G10 | `btype` value space | `btype` is 2 bits, so `{0,1,2,3}` is the complete set: `3` is row #6, `0/1/2` are the Phase B rows | `err_btype3` + `cfg08..cfg30` | [x] |
| G11 | one step past the documented symbol ranges: length symbols `286`,`287` and distance symbols `30`,`31` (`cp_len_base[29]=cp_len_base[30]=0`, `cp_dist_base[30]=cp_dist_base[31]=0`) | `length` and `distance` become just the extra bits (`0`), so `out - 0 >= begin` holds and a zero-length copy with `src == dst` is performed — **not** an error | `cfg21_reserved_len_dist_symbols` | [x] |
| G12 | every exported table mutated to in-range values | the library must read the tables **live** from the exported symbols, not from private copies | `cfg33`, `cfg34`, `cfg35_36` | [x] |
| G13 | `cp_error_reason` initial value | NULL (`.bss`) in both libraries, and never cleared by either | `cfg01_table_contents` + every inflate group | [x] |
| G14 | inputs on which the C library **does not terminate** | `cp_dynamic`'s frame overshoot can make `n` snap back to 256 so that the inner run loop never ends; the Rust build reproduces the same non-termination (both hit the harness' per-case SIGALRM) | `ovs_c_nonterminating`, `ovs_d_transition_band`, 12 of `fuzz38` | [x] |
