# ERRORS.md — error-surface table (Phase A / Phase C)

Mechanically derived from every rejection point in `c_src/src/lib.c`.
There are no `assert`s, no error enums, no `errno` use and no range checks other
than the ones below; the *only* failure sentinel in the whole library is a
`NULL` return from `decode_base64` (`lib.c:42` — "Returns NULL in case of
error"). Every `return (NULL);` statement in the C is one row, plus one row for
the single reachable input on which the C itself has no defined behaviour.

Every row is verified by a differential test that calls **both** shared objects
through their exported `decode_base64` symbol and asserts the *same* sentinel
(not merely "both failed somehow").

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| 1 | `decode_base64` | `src == NULL` (`lib.c:46`, first conjunct of `if (src && *src)` false) → `lib.c:112` | returns `NULL`, nothing allocated | `error_paths.rs::e1_null_pointer` | [x] |
| 2 | `decode_base64` | `src != NULL` but `*src == '\0'` (empty string, second conjunct false) → `lib.c:112` | returns `NULL`, nothing allocated | `error_paths.rs::e2_empty_string` | [x] |
| 3 | `decode_base64` | `calloc(sizeof(char), l + 13)` returns `NULL` (`lib.c:53-56`) — the destination allocation fails | returns `NULL` (nothing to free) | `alloc_wraparound.rs::e3_calloc_fails_deterministic` (via `l <= -14`, so the `int`→`size_t` conversion asks for ~2^64 bytes) **and** `alloc_failure.rs::e3_calloc_fails_under_rlimit` (64 MiB request, address space exhausted under `RLIMIT_AS`, with probes proving `calloc` is the failing call) | [x] |
| 3b | `decode_base64` | same `calloc` check, reached through the `int` overflow of `int l = strlen(src) + 1` (`lib.c:49`): for `strlen(src) >= INT_MAX - 12` the `int` expression `l + 13` is negative, so the implicit conversion to `size_t` yields a huge value | returns `NULL` | `alloc_wraparound.rs::e3b_int_overflow_makes_calloc_fail` (`strlen` = `INT_MAX`, `INT_MAX-1`, `INT_MAX-11`, `INT_MAX-12`) | [x] |
| 4 | `decode_base64` | `malloc(l)` returns `NULL` *after* `calloc` succeeded (`lib.c:60-64`) | `free(dest)`, then returns `NULL` | `alloc_wraparound.rs::e4_malloc_fails_after_calloc_succeeds_deterministic` (`strlen` = 4294967282…4294967294 ⇒ `l` = -13…-1, so `calloc(1, 0..12)` *succeeds* and `malloc((size_t)-13)` *always* fails) **and** `alloc_failure.rs::e4_malloc_fails_under_rlimit` (probes prove `calloc` succeeds and `malloc` fails) | [x] |
| 5 | `decode_base64` | `strlen(src) == 4294967295` ⇒ `l == 0`: **both** NULL checks pass (`calloc(1, 13)` and `malloc(0)` succeed) and the filter loop (`lib.c:67-71`) then writes `strlen(src)` bytes into a zero-byte buffer — the C invokes undefined behaviour on a reachable input | dies from `SIGSEGV` (measured) | `ub_l_zero.rs::e5_l_zero_undefined_behaviour_matches` (each call in a forked child; asserts identical signal *and* exit code) | [x] |

## Generic FFI-boundary cases (Phase C requirement)

| case | test | status |
|------|------|--------|
| `NULL` pointer | `e1_null_pointer` (also interleaved with valid calls) | [x] |
| zero length (`""`), 1-byte NUL-only buffer, leading NUL | `e2_empty_string` | [x] |
| interior NUL — the length "lies"; must stop at the first NUL and equal the truncated string's result | `error_paths.rs::g1_interior_nul_truncates`, `valid_paths.rs::b23_interior_nul_buffer` | [x] |
| every single-byte input `0x01..=0xFF`, ×1…×5 repetitions (covers `'A'-1`, `'Z'+1`, `'a'-1`, `'z'+1`, `'0'-1`, `'9'+1`, `'+'±1`, `'/'±1`, `'='±1`, DEL and every negative `char`) | `error_paths.rs::g2_all_single_bytes` | [x] |
| one step past every documented range boundary, in every group position `k%4`, alone / in pairs / mixed with valid data | `error_paths.rs::g3_range_boundaries_in_all_positions` | [x] |
| oversized lengths: 64 KiB…1 MiB, 2 GiB (`int` overflow), 4 GiB (`int` wraparound to a negative/zero `l`) | `g4_large_but_empty_after_filtering`, `b20_large_inputs`, `e3b_*`, `e4_*_deterministic`, `e5_l_zero_*` | [x] |
| out-of-range **enum** values across the FFI boundary | **N/A** — the ABI has no `enum`, no flags, no mode and no integer parameter: `char *decode_base64(const char *)` takes one pointer. The analogous "any bit pattern is a legal input" surface is the byte domain of the string, covered exhaustively by `g2_all_single_bytes` (all 255 byte values), `b01_exhaustive_single_byte`, `b02_exhaustive_alphabet_pairs` and `b03_exhaustive_alphabet_triples` (all 65³ alphabet triples). | [x] |

## Non-rejections (deliberately *not* errors — verified as valid paths instead)

Asserting an error for any of these would be wrong; they are covered by
`CONFIGS.md` (Phase B).

| condition | C behaviour |
|-----------|-------------|
| bytes outside the base64 alphabet (incl. `0x80..0xFF`, whitespace, control chars) | silently skipped — "Ignore non base64 chars as per the POSIX standard" (`lib.c:66-71`) |
| a string made *only* of non-alphabet bytes (`l == 0` after filtering) | decode loop body never runs; returns a **non-NULL**, all-zero buffer |
| input length not a multiple of 4 / missing padding | the missing positions keep their `'A'` default (`lib.c:74`, `79-89`) and are decoded |
| `'='` anywhere (first, middle, repeated, more than two) | never rejected; `decode('=') == 63`; only `c3` (`lib.c:98`) and `c4` (`lib.c:102`) suppress a byte write |
| `'-'`, `'_'` (URL-safe alphabet) | rejected by `is_base64` ⇒ skipped, not an error |
| decoded payload containing NUL bytes | written verbatim; the buffer is `calloc`-zeroed, so the *C string* ends early while the buffer keeps the rest |

## Environment note (relevant to rows 3, 3b and 4)

`RLIMIT_DATA` on the verification host is 6 GiB, so a 2 GiB-scale allocation
cannot succeed while a 2 GiB input string is held. Both implementations are
subject to the identical limit — they are loaded into the *same* process — so the
differential comparison stays valid; it simply means that for `strlen` near
`INT_MAX` the C returns `NULL` from row 3/3b *or* row 4 depending on the exact
length, and the Rust does the same in every case (verified for
`strlen ∈ {INT_MAX-14 … INT_MAX}`).
