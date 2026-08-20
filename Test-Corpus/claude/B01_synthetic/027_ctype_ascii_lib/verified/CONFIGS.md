# CONFIGS.md — configuration-surface table (Phase A, gates Phase B)

## Mechanical derivation of the axes

Public API (`c_src/include/driver.h`) — the complete set of entry points:

```c
void driver(char c);          /* the only public symbol; nm -D agrees */
```

`driver` takes no options struct, no flags, no mode enum, and the C source
contains **zero** `if`/`switch`/`#ifdef` branches of its own (see
`ERRORS.md`). So the configuration axes cannot be read off `driver`'s own
control flow — they are the things the *code it invokes* branches on. Grepping
what `driver.c` actually calls:

```sh
grep -oE "\b(setlocale|printf|is[a-z]+|to[a-z]+)\b *\(" c_src/src/driver.c | sort -u
# -> isalnum isalpha isblank iscntrl isdigit isgraph islower isprint ispunct
#    isspace isupper isxdigit printf setlocale tolower toupper
```

That yields four real axes:

* **Axis 1 — low-level entry point (14 of them).** The 14 `<ctype.h>`
  interfaces are the lowest-level entry points reachable through this API, one
  per output line. `driver` is the one-shot convenience wrapper over them.
  Tests therefore compare **line by line** (`alphanumeric:`, `alphabetic:`,
  `lowercase:`, `uppercase:`, `digit:`, `hexadecimal:`, `control:`,
  `graphical:`, `space:`, `blank:`, `printing:`, `punctuation:`, `to lower:`,
  `to upper:`), not just the concatenated blob, so a divergence is attributed
  to the specific interface that produced it.
* **Axis 2 — input shape.** The `char` parameter's value class. glibc's tables
  branch on it implicitly; the load-bearing distinction in the translation is
  `(int) c` **sign-extending**, i.e. `0x80 ..= 0xFF` become *negative* table
  indices `-128 ..= -1`. Classes: NUL, C0 control, whitespace (`\t\n\v\f\r`),
  `' '`, digit, upper, lower, hex letter, punctuation, DEL, high-bit/negative,
  plus each class boundary ±1.
* **Axis 3 — locale state, the one true runtime "option".** `driver` calls
  `setlocale(LC_ALL, "C")`, and every `<ctype.h>` interface it then uses reads
  a *locale-dependent* table (`__ctype_b_loc`, `__ctype_tolower_loc`,
  `__ctype_toupper_loc`). Three distinct states the caller can arrange, which
  the code treats differently:
  1. global locale `"C"` (process default) — `setlocale` is a no-op;
  2. global locale pre-set to something else — `setlocale(LC_ALL,"C")`
     *overrides* it, so output must fall back to the `"C"` tables;
  3. a per-thread locale installed with `uselocale()` — this **wins over**
     `setlocale`, so the tables are the thread's, producing genuinely
     non-`"C"` classifications and non-ASCII case mappings (in particular for
     the negative/high-bit indices). This is the code path where a
     sign-extension or table-indexing bug becomes visible in the *values*.
     Locales exercised: `C`, `C.utf8`, `en_US.iso88591`, `en_US.utf8`,
     `de_DE.iso88591`, `tr_TR.iso88599`, `ru_RU.koi8r`, `ja_JP.eucjp`.
* **Axis 4 — output-stream shape.** `printf` writes to the shared
  `FILE *stdout`, so the observable result depends on the stream's buffering
  mode and destination: regular file (fully buffered), pipe, `_IONBF`,
  `_IOLBF`. Also the caller's own `printf` interleaving with `driver`'s, and
  calls from a non-main thread (the locale pointer and stream lock are
  thread-aware).

Plus multiplicity (empty / one / many): 1 call, the same call repeated, and
random-length sequences of different chars inside a single capture — and the
**side effects** `driver` leaves behind (the global locale it mutates).

## Configuration table

Every row is checked by comparing the C `.so` and the Rust `.so` through
`libloading` under byte-for-byte-identical setup, with a fixed-seed PRNG
(`SplitMix64`, seed `0x5D1F_C0DE_1234_5678`) driving the randomized inputs.
"exhaustive 256" means all 256 `char` bit patterns `-128 ..= 127`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| B1 | `driver` | global locale `"C"` (default) · exhaustive 256 · one call per capture · stdout = regular file, fully buffered · whole-blob byte compare | [x] |
| B2 | 14 ctype interfaces, line by line | as B1, but each of the 14 output lines compared individually so a divergence is attributed to `isalnum`/…/`toupper` | [x] |
| B3 | `driver` | global locale `"C"` · 2000 seeded random `char` draws (value-dependent paths, random order) | [x] |
| B4 | `driver` | global locale `"C"` · 200 seeded random *sequences* of 1..16 chars, all calls inside ONE capture (multiplicity + `setlocale` idempotence + buffering/ordering) | [x] |
| B5 | `driver` | global locale **pre-set** to each of 7 foreign locales via `setlocale(LC_ALL, …)` · exhaustive 256 · asserts C≡Rust *and* that both fall back to the `"C"` baseline of B1 | [x] |
| B6 | `driver` | **thread** locale installed with `uselocale(newlocale(LC_ALL_MASK, …))` for each of 8 locales · exhaustive 256 (full 8 × 256 cross-product, per-line compare) | [x] |
| B7 | `driver` | thread locale `tr_TR.iso88599` · exhaustive 256 · additionally asserts the output *differs* from the `"C"` baseline (proves the row really exercises a different table: Turkish dotless-i case mapping + printable high bytes) | [x] |
| B8 | `driver` | called from a **spawned non-main thread**, global locale `"C"` · 256 seeded random draws | [x] |
| B9 | `driver` | called from a spawned non-main thread that installs its **own** `uselocale` (`ru_RU.koi8r`) · exhaustive 256 · high-bit → negative index with non-identity Cyrillic case mapping | [x] |
| B10 | `driver` + caller's own `printf` | global locale `"C"` · caller prints its own marker lines before/between/after `driver` calls in one capture · 100 seeded random chars · checks interleaving order in the shared `stdout` | [x] |
| B11 | `driver` | stdout set to **unbuffered** (`setvbuf(_IONBF)`) · 64 seeded random chars in one capture | [x] |
| B12 | `driver` | stdout set to **line-buffered** (`setvbuf(_IOLBF)`) · 64 seeded random chars in one capture | [x] |
| B12b | `driver` | stdout set to **explicitly fully buffered** (`setvbuf(_IOFBF)`) · 64 seeded random chars in one capture | [x] |
| B13 | `driver` | stdout is a **pipe** (not a regular file) · 32 seeded random chars | [x] |
| B14 | `driver` | side effect: with a foreign global locale pre-set, `setlocale(LC_ALL, NULL)` **after** the call must report the same string for C and Rust (`"C"`) | [x] |
| B15 | `driver` | side effect: with a thread locale installed, `uselocale(NULL)` after the call must be unchanged and identical for C and Rust (i.e. neither implementation clobbers the thread locale) | [x] |
| B16 | `driver` | explicit **boundary** values (`0`, `1`, `8`, `9`, `10`, `13`, `31`, `32`, `47`, `48`, `57`, `58`, `64`, `65`, `70`, `71`, `90`, `91`, `96`, `97`, `102`, `103`, `122`, `123`, `126`, `127`, `-128`, `-127`, `-2`, `-1`) × all 8 locale states (as thread locale) | [x] |
| B17 | `driver` (ABI shape) | symbol called through a `void driver(int)` prototype with all 256 in-range patterns (`0..=255` and `-128..=-1`) — same value arriving as a widened `int`; asserts identical truncation to `char` | [x] |
| B18 | `driver` | the **same** char 10× in one capture (idempotence: output is exactly 10 identical 14-line blocks, no state drift) · 32 seeded chars | [x] |
| B19 | `driver` | thread locale **switched between calls** inside one capture (`C` → `tr_TR.iso88599` → `ja_JP.eucjp` → `C.utf8` → back), same char each time · 64 seeded chars | [x] |
| B20 | 14 ctype interfaces, line by line | full **8 locales × 256 chars** sweep as one systematic cross-product, per-line compare, both `.so`s — the superset sanity sweep | [x] |
| B21 | `driver` | **concurrent** calls: 4 threads × 50 calls sharing `stdout` and the global locale `driver` mutates; line order is nondeterministic so the *multiset* of the 2800 lines is compared | [x] |
| B22 | `driver` | 2000 calls in one capture (~370 KiB) — crosses the stdio buffer boundary hundreds of times | [x] |

## Row → test mapping

Every checkbox above is a real, runnable test; nothing is checked by inspection.

| rows | test file | test function(s) |
|---|---|---|
| B1–B9 | `tests/phase_b_valid_core.rs` | `b1_default_locale_exhaustive_whole_blob`, `b2_every_ctype_interface_line_by_line_exhaustive`, `b3_default_locale_randomized_draws`, `b4_random_sequences_in_one_capture`, `b5_foreign_global_locale_falls_back_to_c`, `b6_foreign_thread_locale_cross_product`, `b7_turkish_thread_locale_differs_from_c_baseline`, `b8_called_from_non_main_thread`, `b9_non_main_thread_with_own_thread_locale` |
| B10–B22 | `tests/phase_b_valid_streams.rs` | `b10_interleaved_with_caller_printf`, `b11_unbuffered_stdout`, `b12_line_buffered_stdout`, `b12b_explicitly_fully_buffered_stdout`, `b13_stdout_is_a_pipe`, `b14_global_locale_after_the_call`, `b15_thread_locale_survives_the_call`, `b16_class_boundaries_under_every_locale`, `b17_char_argument_arriving_as_a_widened_int`, `b18_repeated_identical_calls_are_idempotent`, `b19_thread_locale_switched_between_calls`, `b20_full_locale_char_cross_product`, `b21_concurrent_calls_from_many_threads`, `b22_large_volume_crosses_buffer_boundaries` |
| harness self-check | `tests/smoke.rs` | 3 tests proving the capture is not vacuous (non-empty, 14 labelled lines, different inputs give different bytes) |

## Feature / build configurations

`Cargo.toml` has **no `[features]` table**, so the complete set of valid
feature combinations is the power set of the empty set — one combo:

| # | combo | `cargo` invocation | status |
|---|-------|--------------------|--------|
| F1 | default (empty) | `cargo check/test --no-default-features` | [x] |
| F2 | default (empty), stated explicitly | `cargo check/test --no-default-features --features ""` | [x] (identical to F1) |

`c_src/CMakeLists.txt` likewise defines no options, no
`target_compile_definitions`, and `driver.c` contains no `#ifdef`s, so the C
side has exactly one configuration too.

Because a single feature combo is a thin notion of "every configuration", the
sweep in `run_all_configs.sh` also covers the build configurations that
genuinely change generated code:

| # | build configuration | why it is a distinct code path | status |
|---|---------------------|--------------------------------|--------|
| F3 | Rust **dev** profile cdylib (`target/debug/libdriver.so`) | unoptimised; `panic = unwind` | [x] |
| F4 | Rust **release** profile cdylib (`target/release/libdriver.so`, `opt-level = 3`, `panic = "abort"`) | LLVM is free to exploit parameter ABI attributes and to inline the table lookups — **this configuration is where the one real divergence was found** (see below) | [x] |
| F5 | C reference at `-O0`, `-O1`, `-O2`, `-O3`, `-Os` (built into `target/c-variants/`, `c_src/` untouched) | glibc's `<ctype.h>` swaps `tolower`/`toupper` between an out-of-line call and an inline table lookup on `__OPTIMIZE__`, so the reference itself has two shapes | [x] |

Every row B1–B22 and every `ERRORS.md` row is run under each of these.

## Divergence found and fixed by this sweep

Row **B17/E5** (a `char` argument arriving as a widened `int`) passed against
the dev-profile Rust `.so` but **failed against the release-profile one**:

```text
[E5] out-of-range int 256 (0x00000100): line 6 (interface `control`) differs:
  C   : control: 2
  Rust: control: 0
```

Cause, from the disassembly:

| build | parameter handling |
|---|---|
| C, every `-O` level | `mov %edi,%eax` / `mov %al,-0x4(%rbp)` / `movsbq -0x4(%rbp),%rax` — only the **low byte** is ever used |
| Rust dev | `mov %dil,%al` — low byte, matches |
| Rust release (before the fix) | `mov %edi,%ebx` / `movslq %ebx,%rbx` — the **full 32 bits** became the `<ctype.h>` table index, reading outside the `-128 ..= 255` range glibc's tables define |

Declaring the exported parameter as `c_char` gives LLVM the `signext`
attribute, which lets an optimised build assume the caller already sign-extended
and skip the narrowing. The fix (`src/lib.rs`) is to accept the widened
`c_int` that the C ABI actually delivers and narrow it explicitly
(`c as u8 as c_char`), reproducing the C callee's `mov %al` for every possible
argument while staying ABI-identical for callers using the correct `char`
prototype. The release build now emits `movsbq %bl,%rbx`, and B17/E5 pass in
every configuration.
