# CONFIGS.md — configuration-surface table (Phase A / gate for Phase B)

## Public entry points (complete set)

`include/lib.h` declares exactly one function, and `nm -D` confirms it is the
only exported symbol:

```c
char *encode_base64(int size, const char *src);
```

There is **no** convenience/one-shot wrapper layered over a lower-level API here
— `encode_base64` *is* the lowest-level public entry point. The only other
function in the library, `static char encode(unsigned char)`, is file-static and
therefore reachable only indirectly, through the byte values in `src`. Its five
branches are consequently an input-shape axis of `encode_base64` (axis D below),
not a separate entry point.

## Axes the C actually branches on

Derived from every `if` / loop condition in `c_src/src/lib.c`. There are no
runtime flags, no mode/option setters, no global state, and no `#ifdef`s — the
whole configuration surface is carried by the two parameters.

| axis | source | values |
|------|--------|--------|
| **A. `src` nullity** | `if (!src)` — lib.c:33 | NULL / non-NULL *(NULL is an error row; see `ERRORS.md`)* |
| **B. length mode** | `if (!size) size = strlen(src)` — lib.c:37 | `size == 0` → implicit `strlen` mode; `size != 0` → explicit-length mode |
| **C. `size mod 3`** | `i + 1 < size` / `i + 2 < size` — lib.c:53, 57, 69, 75 | `0` (no padding), `1` (`b2`,`b3` zero-filled → `"=="`), `2` (`b3` zero-filled → `"="`) |
| **D. byte values → `encode()` branch** | `u < 26` / `u < 52` / `u < 62` / `u == 62` / else — lib.c:8–21 | `A`–`Z` / `a`–`z` / `0`–`9` / `+` / `/` — all five must be produced |
| **E. sign bit of the bytes** | `b1 = src[i]` assigns a *signed* `char` into `unsigned char` — lib.c:51–59 | bytes `< 0x80` only / bytes `>= 0x80` present (required to reach 6-bit indices `>= 0x20`) |
| **F. embedded NUL bytes** | interaction of axis B with `strlen` | present / absent — distinguishes the two length modes |
| **G. count** | `for (i = 0; i < size; i += 3)` — lib.c:48 | empty (0) / one byte / two / exactly three (one full group) / many (multi-group) |
| **H. `size` vs the real buffer length** | never validated by the C | `size == len` / `size < len` (deliberate truncation) |
| **I. `size` sign** | loop condition — lib.c:48 | positive / negative *(negative is an `ERRORS.md` row: silently yields an empty result)* |

## Rows — the pruned cross-product

Each row is a configuration the C treats differently. Every row is driven with
**many randomized inputs** (seeded, deterministic `SplitMix64`), not one
hand-picked value, and asserted byte-for-byte against the C — comparing both the
`NULL`-ness of the result and the full NUL-terminated output string.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `encode_base64` | explicit length, `size == len`, `len % 3 == 0`, ASCII bytes only, no NULs, multi-group | [x] |
| 2 | `encode_base64` | explicit length, `size == len`, `len % 3 == 1`, ASCII bytes only — exercises the `"=="` double-padding branch (lib.c:71, 77) | [x] |
| 3 | `encode_base64` | explicit length, `size == len`, `len % 3 == 2`, ASCII bytes only — exercises the single `"="` padding branch (lib.c:77) | [x] |
| 4 | `encode_base64` | explicit length, `size == len == 1` — smallest group, both padding branches taken | [x] |
| 5 | `encode_base64` | explicit length, `size == len == 2` | [x] |
| 6 | `encode_base64` | explicit length, `size == len == 3` — exactly one complete group, no padding | [x] |
| 7 | `encode_base64` | explicit length, full-byte-range random bytes `0x00..=0xFF` (axis E: high-bit/negative `char`s), all `len % 3` classes, lengths 1..=64 | [x] |
| 8 | `encode_base64` | explicit length, bytes chosen to force every `encode()` branch: 6-bit indices `0..=63` swept exhaustively (`A`–`Z`, `a`–`z`, `0`–`9`, `+`, `/`) | [x] |
| 9 | `encode_base64` | explicit length, buffer contains **embedded NUL bytes** (axis F) — must encode past them, unlike `strlen` mode | [x] |
| 10 | `encode_base64` | explicit length, all-`0x00` buffer (min byte value) → all-`'A'` output | [x] |
| 11 | `encode_base64` | explicit length, all-`0xFF` buffer (max byte value) → exercises `u == 62`/`u == 63` | [x] |
| 12 | `encode_base64` | explicit length, `size < len` (axis H truncation): random `size` in `1..=len`, random bytes | [x] |
| 13 | `encode_base64` | **implicit `strlen` mode** (`size == 0`), non-empty NUL-terminated ASCII string, all `strlen % 3` classes | [x] |
| 14 | `encode_base64` | implicit `strlen` mode, NUL-terminated string of random **non-zero** bytes `0x01..=0xFF` (high-bit bytes in `strlen` mode) | [x] |
| 15 | `encode_base64` | implicit `strlen` mode, empty string `""` → `strlen == 0`, loop never runs, 4-byte zeroed buffer | [x] |
| 16 | `encode_base64` | implicit `strlen` mode where bytes follow the NUL — confirms both stop at the NUL and ignore the trailing bytes | [x] |
| 17 | `encode_base64` | explicit length, long inputs (1 KiB–8 KiB, random `len % 3`, random bytes) — multi-group loop at scale, buffer-capacity edge | [x] |
| 18 | `encode_base64` | explicit length, lengths swept **exhaustively** `1..=200` with random bytes — every `len % 3` × every capacity-rounding case | [x] |
| 19 | `encode_base64` | negative `size` (axis I) with a valid `src`: `-1`, `-2`, `-3`, `INT_MIN`, large negatives — empty result, not `NULL` | [x] |
| 20 | `encode_base64` | `size` at/around the signed-overflow threshold. Safely observable only in the band `[0x2000_0000, 0x3FFF_FFFC]` (cap negative → `calloc` fails → `NULL`) and just below it at `0x1FFF_FFFF` with a **real 512 MiB source buffer**. Above `0x3FFF_FFFC` cap wraps to a small positive value and the C writes gigabytes past a 3-byte buffer — ERRORS.md row 7, excluded | [x] |
| 20b | `encode_base64` | explicit length, 64 MiB random input, output compared byte-for-byte — bridges the KiB scale of row 17 and the threshold of row 20 | [x] |
| 21 | `encode_base64` | **whole-buffer** parity: the full `cap`-byte allocation is compared, not just the NUL-terminated prefix, across lengths `1..=300` in both length modes. The C never writes a terminator — it relies on `calloc` zeroing the tail — so a translation that wrote its own NUL, or used `malloc` + partial fill, would pass every prefix test and fail here | [x] |
| 22 | `encode_base64` | `strlen`-result **truncated to `int` with a sign flip**: a 2 GiB string makes `strlen` return `2^31`, which truncates to `INT_MIN`, wraps `size*4` to `0`, and yields `cap == 4` — so a 2 GiB input encodes to the *empty* string. Also `strlen == 0x2000_0000` (→ `NULL`) and `strlen == 0x1000_0001` (→ succeeds). This is the single most translation-sensitive line in the library | [x] |

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** — no default features,
no optional dependencies, hence exactly one feature combination (the empty set).
`verify.sh` derives the powerset from `Cargo.toml` mechanically rather than
assuming this, so it stays correct if features are ever added. Every row is run
against **both** the release and the debug Rust cdylib; the debug build has
overflow checks enabled, which would trap any arithmetic the translation failed
to make explicitly `wrapping_*`.

## Binary executable

Neither build produces a driver binary: `c_src/CMakeLists.txt` contains only
`add_library(driver SHARED src/lib.c)` (0 occurrences of `add_executable`), and
`translation/Cargo.toml` declares only `crate-type = ["cdylib"]` with no
`src/main.rs` and 0 occurrences of `[[bin]]`. Confirmed by inspecting the build
outputs: no `target/{release,debug}/driver` executable is produced. The
stdout-comparison requirement is therefore N/A.

## Row status

All 22 rows pass. Every row drives both `.so` files through `libloading` and
compares byte-for-byte; the randomized rows use a fixed `SplitMix64` seed. Row
coverage totals roughly 100k differential calls per run.

Test mapping: rows 1–20 → `tests/phase_b_valid_paths.rs`
(`cfg_row<N>_*`), rows 21–22 → `tests/phase_b_hardening.rs`.
`harness_agrees_with_reference_encoder` additionally checks both `.so` files
against an independent transliteration of the C plus the RFC 4648 `f`/`fo`/`foo`
/`foob`/`fooba`/`foobar` known-answer vectors, so the differential comparison
cannot pass by both sides failing identically.
