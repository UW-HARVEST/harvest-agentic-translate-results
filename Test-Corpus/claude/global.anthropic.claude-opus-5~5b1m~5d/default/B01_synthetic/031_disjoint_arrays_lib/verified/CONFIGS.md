# CONFIGS.md — Phase B configuration / valid-input surface table

## Axes derived from the C source

The library has **no** runtime options, flags, modes, `#ifdef`s or global state.
`grep -n 'if\|switch\|#if\|for' c_src/src/driver.c` yields only the branches
already inventoried in `ERRORS.md`. The configuration surface is therefore
purely **input shape**, across the **three** public entry points.

### Entry points (full set, lowest level first)

| level | symbol | why it must be driven directly |
|-------|--------|--------------------------------|
| 0 (lowest) | `fma_array` | the actual kernel; `call_fma` only ever calls it with `mul1 = all-ones` and `add = all-zeros`, so the general `mul1*mul2+add` path is **unreachable** from the wrappers and would be untested by wrapper-only tests |
| 1 | `call_fma`  | allocates the three VLAs and forwards to `fma_array`; only `len` and `data` vary |
| 2 (convenience) | `driver` | the one-shot wrapper declared in `driver.h`: parse text → `call_fma` → `printf` |

### Shape axes

* `len`: `0`, `1`, `2`, `3`, small (2..16), boundary-ish (99, 100, 101), larger (1000)
* element values: zeros, ones, small positives, negatives, mixed sign,
  `INT_MIN`, `INT_MAX`, values chosen to overflow `mul1*mul2`, values chosen to
  overflow the `+add`, random full-range `i32`
* `out` buffer state: pre-poisoned with a sentinel, to prove exactly `len`
  elements are written and no more (tail must stay untouched)
* `driver` text shapes: separator kind (space / tab / newline / `\r` / `\v` /
  `\f` / mixed / run-of-many / none-needed-because-sign), sign prefix
  (`+`/`-`/none), leading zeros, leading whitespace, trailing whitespace,
  trailing garbage, count (0, 1, 2, 99, 100, 101, 150), token magnitude
  (in-range / clamping), embedded NUL

## Rows (cross-product pruned to what the C actually distinguishes)

Every row is exercised with **many randomized inputs** (fixed seed
`0x5EED_C0DE`, xorshift64* PRNG in `tests/common/mod.rs`) unless the row is a
single fixed boundary shape. Both the C `.so` and the Rust `.so` are loaded with
`libloading` and their outputs compared byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `fma_array` | `len == 1`, random `mul1`/`mul2`/`add` in a safe (non-overflowing) range; `out` poisoned, tail checked | [x] |
| C2 | `fma_array` | `len == 2` and `len == 3` (smallest multi-element shapes), random small values | [x] |
| C3 | `fma_array` | `len` random in 4..=64, random small values, `out` poisoned + tail checked | [x] |
| C4 | `fma_array` | `len == 100` and `len == 1000` (large shape; exercises any vectorized/unrolled loop tail) | [x] |
| C5 | `fma_array` | all elements zero; then all elements one; then `add` all-zero + `mul1` all-one (the exact shape `call_fma` uses) — isolates the identity path | [x] |
| C6 | `fma_array` | mixed-sign and all-negative operands, random in `-1000..=1000` | [x] |
| C7 | `fma_array` | **overflow shapes**: operands drawn from `{INT_MIN, INT_MAX, INT_MIN+1, INT_MAX-1, -1, 0, 1, 2, 65536, -65536, 46341, -46341}` so `mul1*mul2` and/or `+add` wrap; asserts wrapping codegen matches | [x] |
| C8 | `fma_array` | fully random `i32` operands over the whole range (property-style, 4000 vectors) | [x] |
| C9 | `fma_array` | `len` argument smaller than the buffers (partial write): asserts only the first `len` elements of `out` differ from the poison in both | [x] |
| C10 | `call_fma` | `len == 1`, random `data[0]` over full `i32` range | [x] |
| C11 | `call_fma` | `len` random in 2..=16, random `data` over full `i32` range (return must be `data[len-1]`) | [x] |
| C12 | `call_fma` | `len == 99`, `100`, `101` (the shapes `driver` can produce, incl. its cap boundary) | [x] |
| C13 | `call_fma` | `len == 1000` (VLA larger than the `driver` cap, still stack-safe) | [x] |
| C14 | `call_fma` | `data` containing `INT_MIN`/`INT_MAX`/`0`/`-1` at the **last** index (the returned element) and at index 0 | [x] |
| C15 | `call_fma` | `data` buffer longer than `len` — asserts the trailing elements are ignored | [x] |
| C16 | `driver` | single integer, no whitespace: random value in `i32` range, rendered decimal | [x] |
| C17 | `driver` | single integer with an explicit `+` sign; and with leading zeros (`"007"`, `"+000042"`) | [x] |
| C18 | `driver` | two integers separated by one space; random values | [x] |
| C19 | `driver` | `n` random integers (n in 2..=20) separated by a single space | [x] |
| C20 | `driver` | `n` integers separated by **tabs**; by **newlines**; by `\r`; by `\v`; by `\f` — each of the whitespace classes `%d` skips | [x] |
| C21 | `driver` | `n` integers separated by **runs** of mixed whitespace (e.g. `" \t\n  "`), random run lengths | [x] |
| C22 | `driver` | leading whitespace before the first integer; trailing whitespace after the last | [x] |
| C23 | `driver` | negative integers with **no** separator before the sign (`"5-3-2"`) — `%d` consumes `-3` as a new token, so the token count differs from the space-separated rendering | [x] |
| C24 | `driver` | exactly **99** integers (one below the cap) | [x] |
| C25 | `driver` | exactly **100** integers (at the cap) | [x] |
| C26 | `driver` | exactly **101** integers (one past the cap — must print the 100th) | [x] |
| C27 | `driver` | **150** and **300** integers (well past the cap) | [x] |
| C28 | `driver` | all tokens `INT_MIN`/`INT_MAX` (`"-2147483648 2147483647"`) — in-range boundary values | [x] |
| C29 | `driver` | tokens that overflow `int` and get **truncated to their low 32 bits** by glibc `%d` (`"2147483648"`→`-2147483648`, `"-2147483649"`→`2147483647`, `"99999999999999999999"`→`-1`), mixed with valid tokens | [x] |
| C30 | `driver` | valid prefix + trailing garbage (`"1 2 3 abc 9"`, `"7 0x10"`, `"4 5 +"`) — early break | [x] |
| C31 | `driver` | fully randomized text: random mix of integers, whitespace runs, and occasional garbage characters (200 cases) — end-to-end fuzz of the composed parse→`call_fma`→`printf` pipeline | [x] |
| C32 | `driver` | repeated invocation (10x in a row) with different inputs in one process — proves no residual global state and that stdout ordering matches | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/driver.c)` —
there is **no** `add_executable`, and `driver.c` has no `main`. Likewise
`translation/Cargo.toml` declares only `[lib] crate-type = ["cdylib"]` with no
`[[bin]]`. **The project builds no binary**, so the "compare C and Rust stdout
of the driver binary" gate is vacuous. `driver`'s `printf` output is instead
compared byte-for-byte by capturing file descriptor 1 around each call
(`tests/common/mod.rs::capture_stdout`), which covers the same ground.

## Test adequacy — mutation check (22 mutants injected into `src/lib.rs`)

To prove the differential suite can actually *fail*, each mutant was built and the
whole suite re-run. `src/lib.rs` was restored and its md5 re-verified
(`814bca90b2443d7609a6d39af97faf10`) after every trial.

**17 / 22 killed.** All 5 survivors are provably semantically equivalent to the C:

| mutant | outcome | why |
|--------|---------|-----|
| `out[n-1]` → `out[0]` | KILLED | |
| `driver` cap `100` → `99` | KILLED | |
| `driver` cap `100` → `101` | KILLED | |
| `wrapping_mul` → `saturating_mul` | KILLED | |
| `wrapping_add` → `saturating_add` | KILLED | |
| `fma_array` loop `i < len` → `i <= len` | KILLED | |
| `fma_array` loop `i < len` → `i < len-1` | KILLED | |
| `matched != 1` → `matched == 0` | KILLED | |
| `if len == 0 { return 0 }` → `return 1` | KILLED | |
| `ones[i] = 1` → `2` | KILLED | |
| `zeros[i] = 0` → `1` | KILLED | |
| `cursor.add(nb)` → `cursor.add(nb+1)` | KILLED | |
| `printf("%d\n")` → `printf("%d")` | KILLED | |
| `sscanf("%d%zn")` → `"%i%zn"` | KILLED | |
| `if len < 0 { return 0 }` → `return 7` | KILLED | |
| `call_fma(.., i)` → `call_fma(.., i-1)` | KILLED | |
| drop the `+ add[i]` term | KILLED | |
| `matched != 1` → `matched < 1` | survived | **equivalent**: `%n`/`%zn` conversions are not counted, so `sscanf("%d%zn", ..)` returns only `-1`, `0` or `1` (verified empirically against glibc). `!= 1` and `< 1` therefore agree on the whole domain. |
| `sscanf("%d%zn")` → `"%d%n"` | survived | **equivalent on this target**: `nb` is re-zeroed every iteration, so a 4-byte `%n` store into the low half of a little-endian `usize` leaves the value unchanged for any `nb < 2^31`. (The translation still uses `%zn` with a `size_t`, exactly matching the C.) |
| `data` init `[0; 100]` → `[1; 100]` | survived | **equivalent**: the C leaves `int data[100]` *uninitialized*; only the first `i` elements — every one written by `sscanf` — are ever read. The initial value is unobservable, which is why the Rust may safely zero it. |
| swap the `mul1`/`mul2` arguments | survived | **equivalent**: integer multiplication is commutative. |
| `out[0] = 0;` → `out[0] = 9;` | survived | **equivalent**: `fma_array` unconditionally overwrites `out[0]` when `len >= 1` (and `len == 0` returns early). This mirrors the same dead store present in the C at driver.c:40. |
