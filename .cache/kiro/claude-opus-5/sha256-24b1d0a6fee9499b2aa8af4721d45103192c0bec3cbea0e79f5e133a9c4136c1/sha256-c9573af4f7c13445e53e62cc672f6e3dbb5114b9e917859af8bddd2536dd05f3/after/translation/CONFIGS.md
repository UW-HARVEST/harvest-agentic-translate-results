# CONFIGS.md — configuration surface (valid inputs) of `c_src/src/lib.c`

Derived mechanically from the branches the C actually takes:

```sh
$ grep -n 'if\s*(\|switch\|#if\|for\s*(' c_src/src/lib.c
8:    if (u < 26)            # encode() branch 1  -> 'A'+u
11:   if (u < 52)            # encode() branch 2  -> 'a'+(u-26)
14:   if (u < 62)            # encode() branch 3  -> '0'+(u-52)
17:   if (u == 62)           # encode() branch 4  -> '+'
      (fallthrough)          # encode() branch 5  -> '/'
33:   if (!src)              # error, see ERRORS.md
37:   if (!size)             # MODE SWITCH: size==0 => strlen(src)
42:   if (!out)              # error, see ERRORS.md
48:   for (i = 0; i < size; i += 3)   # group loop
53:   if (i + 1 < size)      # b2 present?
57:   if (i + 2 < size)      # b3 present?
69:   if (i + 1 < size)      # 3rd char vs '=' pad
75:   if (i + 2 < size)      # 4th char vs '=' pad
```

## Axes

* **Entry points** — there is exactly **one** public entry point, and it is
  also the lowest-level one: `encode_base64(int size, const char *src)`
  (`c_src/include/lib.h:1`). `encode()` is `static`, so it is not reachable
  across the ABI; it is exercised *only* indirectly, which is why axis **V**
  below deliberately drives all five of its branches through crafted bytes.
* **Runtime options / modes / flags** — the library has **no** setters, **no**
  global or thread-local state, **no** `#ifdef`s and **no** enums. The single
  mode switch is line 37: `size == 0` reinterprets `src` as a NUL-terminated
  string (axis **M**).
* **M (length mode)**: `M0` = `size == 0` (strlen mode) · `M+` = `size > 0`
  (explicit length) · `M-` = `size < 0` (loop skipped).
* **R (size mod 3)**: `R0` / `R1` / `R2` — selects the padding branches
  (lines 69/75): `R0` → no `=`, `R1` → `"=="`, `R2` → `"="`.
* **G (group count)**: `G1` = one 3-byte group · `G2` = two · `Gn` = many.
* **V (byte-value class, drives `encode()`)**: `V<26` (`A`–`Z`) · `V26-51`
  (`a`–`z`) · `V52-61` (`0`–`9`) · `V62` (`+`) · `V63` (`/`) · `Vhigh` = bytes
  ≥ 0x80 (negative `signed char` on x86-64, converted to `unsigned char`) ·
  `Vnul` = embedded `0x00` · `Vall` = the full 0..=255 range.
* No byte-order / element-width / format axis exists — the API is byte-oriented.

## Configuration table

Every row is driven with **many pseudo-random inputs** (xorshift64\*, fixed
seed `0x0002_0240_601C_0FEE`) unless the row says "exhaustive". In addition to
the output bytes, every call compares the **exact allocator request**: the test
executable interposes `calloc` (both `.so`s bind their `calloc` relocation to
it, verified by `harness_self_test::interposition_is_active`) and forwards to
`__libc_calloc`, so the number of `calloc` calls and the `(nmemb, size)` pair
are compared directly. That pins down the `size * 4 / 3 + 4` capacity
expression — including its signed-`int` overflow and its sign-extending
conversion to `size_t` — and is also cross-checked against the model in
`cap_of()`. (`malloc_usable_size` was tried first and rejected: it reflects
allocator state, not the requested size, and is not reproducible.)

The comparison covers the **entire** allocated buffer, not just the string up
to the first NUL, so the zero-fill tail is compared too.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| 1  | `encode_base64` | `M+`, `R1`, `G1`, `Vall` — **exhaustive** all 256 one-byte inputs (`size=1`) | [x] |
| 2  | `encode_base64` | `M+`, `R2`, `G1`, `Vall` — **exhaustive** all 65 536 two-byte inputs (`size=2`) | [x] |
| 3  | `encode_base64` | `M+`, `R0`, `G1`, `Vall` — `size=3`, 200 000 randomized triples + 4 096 systematic ones | [x] |
| 4  | `encode_base64` | `M+`, `R1`, `G2`, random bytes — `size=4` | [x] |
| 5  | `encode_base64` | `M+`, `R2`, `G2`, random bytes — `size=5` | [x] |
| 6  | `encode_base64` | `M+`, `R0`, `G2`, random bytes — `size=6` | [x] |
| 7  | `encode_base64` | `M+`, `Gn`, every `size` in `1..=256`, random bytes ×64 each (covers `R0/R1/R2` × all group counts) | [x] |
| 8  | `encode_base64` | `M+`, `Gn`, random `size` in `257..=4096`, random bytes | [x] |
| 9  | `encode_base64` | `M+`, `V<26` — all-`0x00` payload (`encode` branch 1, output all `'A'`) | [x] |
| 10 | `encode_base64` | `M+`, `V63` — all-`0xFF` payload (`encode` branches 4/5, `'/'`) | [x] |
| 11 | `encode_base64` | `M+`, `V62` — bytes crafted so `b4/b5/b6/b7 == 62` exactly (`'+'`) | [x] |
| 12 | `encode_base64` | `M+`, `V52-61` / `V26-51` — bytes crafted to land in the `'a'`–`'z'` and `'0'`–`'9'` branches | [x] |
| 13 | `encode_base64` | `M+`, all 64 sextet values forced into each of the 4 output positions (full `encode()` domain 0..=63 × position) | [x] |
| 14 | `encode_base64` | `M+`, `Vhigh` — payload drawn only from `0x80..=0xFF` (signed-`char` conversion) | [x] |
| 15 | `encode_base64` | `M+`, `Vnul` — random payload with `0x00` bytes interleaved at random positions (NULs must be **encoded**, not terminate) | [x] |
| 16 | `encode_base64` | `M+`, `size` **smaller than** the underlying buffer (only `size` bytes may be read; rest is poison `0xAA`) | [x] |
| 17 | `encode_base64` | `M+`, `size` a large exact multiple of 3 (`3072`), `R0`, `Gn` | [x] |
| 18 | `encode_base64` | `M+`, capacity-tight sizes `1,2,4,7,10,13,…` where `4*ceil(size/3)` is closest to `cap` | [x] |
| 19 | `encode_base64` | `M0` (strlen mode), NUL-terminated ASCII of every length `0..=128` (all `R` classes) | [x] |
| 20 | `encode_base64` | `M0`, NUL-terminated payload containing high-bit bytes `0x80..=0xFF` | [x] |
| 21 | `encode_base64` | `M0`, empty string `""` → `cap==4`, loop skipped | [x] |
| 22 | `encode_base64` | `M0`, content followed by NUL followed by trailing garbage (strlen truncation) | [x] |
| 23 | `encode_base64` | `M-`, `size ∈ {-1,-2}` → allocation succeeds, loop skipped, `""` returned | [x] |
| 24 | `encode_base64` | `M-`, `size == INT_MIN` / `INT_MIN+1` (`size*4` wraps to a non-negative `cap`) → `""` | [x] |
| 25 | `encode_base64` | `M+`, randomized fuzz sweep: 20 000 iterations of random `size ∈ 1..=512` × random bytes × random buffer slack | [x] |

Rows 23–24 overlap with `ERRORS.md` rows 15–17 on purpose: they are the *valid*
(non-`NULL`) side of the negative-`size` boundary.

## Feature combinations

`translation/Cargo.toml` has **no `[features]`** section, so `default` and
`--no-default-features` are the same build. `check_features.sh` enumerates the
feature list from `Cargo.toml` and re-runs the full suite plus the `nm -D`
symbol diff for each combination that exists (two invocations: `default` and
`--no-default-features`).

## Proof that the suite can detect divergence

`mutation_check.sh` applies 10 targeted mutations to `src/lib.rs` (alphabet
boundaries `26`/`62`, the `'+'` and `'='` literals, the `*4` and `+4` capacity
terms, the `>> 2` shift, the `& 0x3f` mask, the loop stride, and the
`size == 0` mode switch), rebuilds, and confirms the suite **fails** for each,
then restores the original and confirms it passes. All 10 are detected.
