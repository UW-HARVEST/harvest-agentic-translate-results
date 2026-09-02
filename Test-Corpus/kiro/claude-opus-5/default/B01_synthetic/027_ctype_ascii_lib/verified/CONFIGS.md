# CONFIGS.md — Configuration-surface table (Phase B gate)

## How the axes were derived

`driver.c` contains no `if`, `switch`, `?:`, or `#ifdef`, so the branching that
matters is not in the C file's own control flow — it is in the state the C file
*reads* and the data-dependent table lookups it performs. Enumerating those
mechanically from the source:

### Axis 1 — public entry points (the FULL set)

`include/driver.h` declares exactly one:

| entry point | signature | notes |
|---|---|---|
| `driver` | `void driver(char c)` | the only public symbol; there is no lower-level variant and no convenience wrapper, so "exercise the low-level entry points too" collapses to this one function — but see Axis 4 for the two *ABI-level* ways to call it |

### Axis 2 — runtime state the C reads (the only "options")

Grepping the C for state it consults:

| line in `driver.c` | state read/written | axis values |
|---|---|---|
| `setlocale(LC_ALL, "C")` | process-**global** locale, unconditionally overwritten to `"C"`; return value discarded | global locale before the call: `C` / a UTF-8 locale / an 8-bit ISO-8859-1 locale |
| `isalnum` … `ispunct` (×12) | `(*__ctype_b_loc())[c]` — the **live** ctype table, read fresh on every call | which locale that table belongs to: the global one, or a **thread** locale installed via `uselocale()` (which `setlocale` cannot displace) |
| `tolower(c)`, `toupper(c)` | glibc functions over the same live locale | same as above; `tr_TR` is the case-mapping outlier |
| all 14 `printf`s | libc `stdout` FILE and its buffering mode | `stdout` a tty (line-buffered) vs. redirected to a file (fully buffered) |

### Axis 3 — input shapes the code special-cases

`c` is a single `char`, so the input "shape" axis is which ctype equivalence
class the value falls into. `char` is **signed** on x86-64 Linux, so the domain
is `-128..=127` and the high-bit bytes index the table at negative offsets. The
classes below are the distinct table rows glibc's `"C"` locale actually
distinguishes (read off the `_IS*` masks), plus the boundary values:

`0x00` · `0x01..0x08` · `0x09` · `0x0A..0x0D` · `0x0E..0x1F` · `0x20` ·
`0x21..0x2F` · `0x30..0x39` · `0x3A..0x40` · `0x41..0x46` · `0x47..0x5A` ·
`0x5B..0x60` · `0x61..0x66` · `0x67..0x7A` · `0x7B..0x7E` · `0x7F` · `0x80` ·
`0x81..0xFE` · `0xFF`

### Axis 4 — ABI-level call shape

| value | why the C distinguishes it |
|---|---|
| argument passed as a `char` | the declared prototype |
| argument passed as a full-width `int` | x86-64 SysV puts a sub-`int` argument in a 32-bit register slot and lets the callee ignore the upper bits; a caller compiled against a different/absent prototype (or an FFI binding) can therefore deliver bits 8..31, and the callee's narrowing must match. This is the same class of input as an out-of-range enum value crossing FFI |

### Axis 5 — call multiplicity / ordering

| value | why it matters |
|---|---|
| one call | baseline |
| many calls, monotone `0..255` order | baseline sweep |
| many calls, seeded-shuffle order | catches order-dependent state (lazily-built tables, cached locale) that a monotone walk hides |
| C and Rust calls interleaved without an intervening flush | both must share the *same* libc `stdout` buffer; a translation using Rust's own `std::io` would interleave differently |

## Table

Cross-product of the axes, pruned to combinations the C treats differently.
Every row is exercised with `SAMPLES = 64` seeded-random draws from its value
range plus both range endpoints (see `tests/common/mod.rs::diff_random_in_range`);
rows whose range is a single value are exercised with that value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` (char) | global locale `C`; `c == 0x00` NUL — `_IScntrl` only | [x] |
| 2 | `driver` (char) | global locale `C`; `c` in `0x01..=0x08` — pure control chars | [x] |
| 3 | `driver` (char) | global locale `C`; `c == 0x09` TAB — `_IScntrl \| _ISspace \| _ISblank` | [x] |
| 4 | `driver` (char) | global locale `C`; `c` in `0x0A..=0x0D` — `\n \v \f \r`, `_IScntrl \| _ISspace`, not blank | [x] |
| 5 | `driver` (char) | global locale `C`; `c` in `0x0E..=0x1F` — pure control chars | [x] |
| 6 | `driver` (char) | global locale `C`; `c == 0x20` SPACE — `_ISblank \| _ISspace \| _ISprint`, **not** `_ISgraph` | [x] |
| 7 | `driver` (char) | global locale `C`; `c` in `0x21..=0x2F` — punctuation | [x] |
| 8 | `driver` (char) | global locale `C`; `c` in `0x30..=0x39` — digits, 5 mask bits at once | [x] |
| 9 | `driver` (char) | global locale `C`; `c` in `0x3A..=0x40` — punctuation | [x] |
| 10 | `driver` (char) | global locale `C`; `c` in `0x41..=0x46` — `A`–`F`, upper **and** `_ISxdigit` | [x] |
| 11 | `driver` (char) | global locale `C`; `c` in `0x47..=0x5A` — `G`–`Z`, upper, not xdigit | [x] |
| 12 | `driver` (char) | global locale `C`; `c` in `0x5B..=0x60` — punctuation | [x] |
| 13 | `driver` (char) | global locale `C`; `c` in `0x61..=0x66` — `a`–`f`, lower **and** `_ISxdigit` | [x] |
| 14 | `driver` (char) | global locale `C`; `c` in `0x67..=0x7A` — `g`–`z`, lower, not xdigit | [x] |
| 15 | `driver` (char) | global locale `C`; `c` in `0x7B..=0x7E` — punctuation | [x] |
| 16 | `driver` (char) | global locale `C`; `c == 0x7F` DEL — `_IScntrl`, largest positive `char` | [x] |
| 17 | `driver` (char) | global locale `C`; `c == 0x80` — most-negative `char` (`-128`), lowest legal table index | [x] |
| 18 | `driver` (char) | global locale `C`; `c` in `0x81..=0xFE` — negative table indices | [x] |
| 19 | `driver` (char) | global locale `C`; `c == 0xFF` — `-1`, the `EOF` slot of the tables | [x] |
| 20 | `driver` (char) | global locale `C`; **all 256** values, monotone order | [x] |
| 21 | `driver` (char) | global locale `C`; all 256 values, seeded-shuffle order | [x] |
| 22 | `driver` (char) | global locale `C`; seeded-random values, whole `0x00..=0xFF` domain, 512 draws | [x] |
| 23 | `driver` (int) | global locale `C`; low byte valid, bits 8..31 zero (`0x0000_0041`) | [x] |
| 24 | `driver` (int) | global locale `C`; low byte valid, bits 8..31 **garbage** (`0xDEAD_BE41`), seeded-random | [x] |
| 25 | `driver` (int) | global locale `C`; `128` and `256` — not representable in `char`, must narrow | [x] |
| 26 | `driver` (int) | global locale `C`; `INT_MIN`, `INT_MAX`, `-1`, `0`, seeded-random full `i32` range | [x] |
| 27 | `driver` (char) | global locale pre-set to a **UTF-8** locale (`C.utf8`, `en_US.utf8`, `de_DE.utf8`); all classes | [x] |
| 28 | `driver` (char) | global locale pre-set to an **ISO-8859-1** locale (`en_US`/`de_DE`/`fr_FR.iso88591`); all 256 values — high bytes are letters there, so `driver`'s own `setlocale(LC_ALL,"C")` must be what wins | [x] |
| 29 | `driver` (char) | **thread** locale (`uselocale`) = ISO-8859-1; all 256 values — `setlocale` cannot displace it, so the live table is *not* the `"C"` one | [x] |
| 30 | `driver` (char) | **thread** locale = UTF-8, incl. `tr_TR.utf8`; all 256 values — Turkish dotted/dotless `I` case mapping | [x] |
| 31 | `driver` (char) | thread locale ISO-8859-1 **and** global locale simultaneously set to a third locale; all 256 values | [x] |
| 32 | `driver` (char) | repeated invocation: same `c` called N times — output must be idempotent (`setlocale` is re-run each time) | [x] |
| 33 | `driver` (char) | C and Rust calls interleaved inside one capture with no flush between — must share one `stdout` FILE buffer | [x] |
| 34 | `driver` (char) | `stdout` fully buffered (redirected to a file) and drained only by the caller — `driver` never flushes | [x] |
| 35 | `driver` (char) | `stdout` buffering set explicitly via `setvbuf` to `_IOFBF`, `_IOLBF` and `_IONBF` — the `printf` ordering axis | [x] |
| 36 | `driver` (char/int) | called from a **non-main thread**, so `__ctype_b_loc()` resolves through a different TLS slot | [x] |

## Test mapping

| rows | test file :: test |
|---|---|
| 1–19 | `tests/configs.rs` :: `cfg_01_nul` … `cfg_19_eof_slot_ff` |
| 20–21 | `tests/smoke.rs` :: `exhaustive_all_256_char_values`, `exhaustive_all_256_shuffled_order` |
| 22 | `tests/configs.rs` :: `cfg_22_random_full_domain` |
| 23–26 | `tests/configs.rs` :: `cfg_23_int_clean_low_byte` … `cfg_26_int_extremes` |
| 27–28 | `tests/locale.rs` :: `cfg_27_global_locale_utf8`, `cfg_27b_setlocale_side_effect_is_observable`, `cfg_28_global_locale_latin1_all_256` |
| 29–30 | `tests/locale.rs` :: `cfg_29_thread_locale_latin1_all_256`, `cfg_30_thread_locale_utf8_all_256`, `cfg_30b_turkish_dotless_i_ground_truth` |
| 31 | `tests/locale.rs` :: `cfg_31_thread_and_global_locale_differ`, `cfg_31b_thread_locale_with_wide_int_args`, `cfg_31c_locale_churn_between_calls` |
| 32–36 | `tests/configs.rs` :: `cfg_32_repeated_calls_are_idempotent` … `cfg_36b_multiple_threads` |

## A note on rows 27–31: differential-test ordering

`diff_char` calls the C first and the Rust second. For any state `driver` itself
*mutates*, that ordering hides bugs: `driver` calls `setlocale(LC_ALL, "C")`, so
by the time the Rust side runs, the C side has already reset the global locale
and the Rust is never actually observed under the caller's locale. A Rust that
omitted `setlocale` entirely would pass.

This was a real blind spot in this suite — `mutation_check.sh` caught it (the
"no setlocale" mutant survived). Rows 27–28 and 31 therefore use
`diff_char_prepared` / `diff_all_chars_prepared`, which re-establish the caller's
locale immediately before **each** side, and `cfg_27b` additionally asserts the
side effect directly by querying `setlocale(LC_ALL, NULL)` after each call.

Thread-locale rows (29–30) do not need this, since `setlocale` cannot displace a
locale installed with `uselocale()`.

## Bugs this phase found and fixed

| # | symptom | root cause | fix |
|---|---------|-----------|-----|
| 1 | under a `uselocale()` thread locale, `driver(0x80)` printed `control: 0` where the C printed `control: 2` | the Rust embedded a frozen copy of the `"C"`-locale ctype tables; the C reads `(*__ctype_b_loc())[c]` **live**, and `setlocale` cannot displace a thread locale | classify through `__ctype_b_loc()` at the same point the C macro does |
| 2 | under `tr_TR`, `tolower('I')` gave `i` where the C gave `I` | the Rust reimplemented the case tables instead of calling glibc's `tolower`/`toupper`, which honour the live locale (dotless `ı` is not a single byte) | call libc `tolower`/`toupper` |
| 3 | **SIGSEGV** when a caller left garbage in bits 8..31 of the argument | `extern "C" fn driver(c: c_char)` makes rustc mark the parameter `signext` and emit `movslq %edi`, indexing the ctype table with the whole 32-bit value; GCC's code keeps only `%al` and re-reads it with `movsbq` | take the argument as `c_int` and narrow explicitly (`c_arg as u8 as c_char`), reproducing the C's `mov %al` + `movsb` |
