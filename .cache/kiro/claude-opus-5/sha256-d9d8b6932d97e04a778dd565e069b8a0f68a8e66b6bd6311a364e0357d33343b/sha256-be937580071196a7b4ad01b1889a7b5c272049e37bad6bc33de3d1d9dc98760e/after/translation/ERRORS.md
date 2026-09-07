# ERRORS.md — error / rejection surface table

Derived mechanically from the C source, not from docs. The grep used:

```
grep -nE 'return -1|return NULL|assert|RETURN_ERROR|errno|exit\(|abort|stderr|return [0-9]|default:|if *\(|switch *\(' c_src/src/*.c c_src/src/*.h
```

Findings: this library has **no error enum, no `RETURN_ERROR` macro, no `assert`,
no null checks, and no `return -1` / `return NULL` anywhere**. The complete
rejection surface is:

1. one explicit argument-count rejection in `main` (`mdmain.c:29-32`),
2. one *silent* rejection inside the macro-generated accumulator — the
   `default: break;` arm of `DISPATCH_REP` (`mdmacros.h:91`), reached through the
   public `use_generated`,
3. `atoi`'s own no-digits / out-of-range behaviour, which `main` feeds into every
   subsequent computation.

Because `op_*`, `helper_call`, `helper_ptr` take plain `int` by value and never
dereference anything, **no input value is rejected** by them — that is itself a
claim under test (rows 11-14): a Rust translation that panics on overflow where C
wraps would be a divergence.

| # | function | trigger (exact invalid input/condition) | expected C result | ✓ |
|---|----------|------------------------------------------|-------------------|---|
| 1 | `main` (driver) | `argc < 3` — no args at all (`argc == 1`) | `fprintf(stderr, "usage: %s A B\n", argv[0])`; exit status **2**; **stdout empty** | [x] |
| 2 | `main` (driver) | `argc < 3` — exactly one arg (`argc == 2`) | same as row 1: usage on stderr, exit **2**, stdout empty | [x] |
| 3 | `main` (driver) | `argc >= 3` but arg 1 has no digits (`"abc"`) | *not* rejected: `atoi` returns `0`, run proceeds with `a == 0`, exit **0** | [x] |
| 4 | `main` (driver) | `argc >= 3` but arg 2 is empty string `""` | *not* rejected: `atoi("") == 0`, exit **0** | [x] |
| 5 | `main` (driver) | arg is `"12abc"` / `"  -7x"` / `"+9"` / `"--3"` / `"0x10"` | `atoi` parses the longest valid prefix (`12`, `-7`, `9`, `0`, `0`); never rejected | [x] |
| 6 | `main` (driver) | arg overflows `int` on the **positive** side (`"99999999999"`, `"9223372036854775808"`) | glibc `atoi` = `(int)strtol(...)`; `strtol` clamps to `LONG_MAX` ⇒ `(int)0x7FFFFFFFFFFFFFFF == -1` | [x] |
| 7 | `main` (driver) | arg overflows `int` on the **negative** side (`"-99999999999"`, `"-9223372036854775809"`) | `strtol` clamps to `LONG_MIN` ⇒ `(int)0x8000000000000000 == 0` | [x] |
| 8 | `main` (driver) | arg is exactly `"-9223372036854775808"` (`LONG_MIN`, representable, no clamp) | `strtol` returns `LONG_MIN` ⇒ `(int)` truncation == `0` | [x] |
| 9 | `main` (driver) | more than 2 args (`argc > 3`) | extra args silently ignored, exit **0** | [x] |
| 10 | `use_generated` | `n` outside the `switch` label set `0..=6` — i.e. `n == 7` | `default: break;` ⇒ accumulator untouched ⇒ returns `INIT_FOR(OP)` (`0` for add/sub, `1` for mul), prints `gen.acc=<INIT>`. Note `REP7` *exists* but the switch never selects it. | [x] |
| 11 | `use_generated` | `n` negative (`-1`, `-100`, `INT_MIN`) | `default: break;` ⇒ returns `INIT_FOR(OP)` | [x] |
| 12 | `use_generated` | `n` far above range (`8`, `100`, `INT_MAX`) | `default: break;` ⇒ returns `INIT_FOR(OP)` | [x] |
| 13 | `op_add` | signed overflow: `INT_MAX + 1`, `INT_MIN + (-1)`, `INT_MAX + INT_MAX` | no rejection; gcc emits wrapping two's-complement `add`. Rust must wrap, **not** panic | [x] |
| 14 | `op_sub` | signed overflow: `INT_MIN - 1`, `INT_MAX - INT_MIN`, `0 - INT_MIN` | no rejection; wrapping `sub` | [x] |
| 15 | `op_mul` | signed overflow: `INT_MAX * 2`, `INT_MIN * -1`, `65536 * 65536` | no rejection; wrapping `imul` (low 32 bits) | [x] |
| 16 | `helper_call` | overflow in `r + acc` (e.g. `a = INT_MAX, b = 0` under `-DOP=add`, `REPEAT >= 1`) | no rejection; wrapping `add` of the two wrapped halves | [x] |
| 17 | `helper_call` / `helper_ptr` / `op_*` | *no* null-pointer surface exists — every parameter is `int` by value | passing arbitrary bit patterns (incl. `0`, `INT_MIN`, `INT_MAX`) is always valid; asserted for completeness | [x] |
| 18 | `G_OP` | the `int (*)(int,int)` global is non-`const` and writable in C | reading it yields `&op_<OP>`; the library itself never reads it, so overwriting it from a caller changes nothing inside the `.so`. Rust `static mut G_OP` must behave identically | [x] |
| 19 | `G_OP_NAME` | `const char *` global | points at a 3-byte + NUL C string equal to `STR(OP)`; never `NULL` | [x] |
| 20 | `use_generated` | out-of-range "enum-like" selector crossing FFI: `n` is a plain `int` used as a `switch` selector, so every one of the 2^32 values is a real input | every value not in `0..=6` maps to `default` ⇒ `INIT_FOR(OP)`; randomized sweep asserts this | [x] |
| 21 | `G_OP` **and** `G_OP_NAME` | an external caller *stores* to either global. The `const` in `const char *G_OP_NAME` qualifies the pointee, not the pointer, so gcc emits **both** globals into writable `.data` (`readelf -S`: section `.data`, `WA`) | the store succeeds and is observable on read-back; the library's own behaviour is unchanged. **A Rust immutable `static` would land in `.data.rel.ro`, which RELRO makes read-only, and the same store would SIGSEGV** — so both are `static mut` | [x] |
| 22 | `G_OP_NAME` pointee | storing *through* the pointer, into the string itself | the target is a `STR(OP)` string literal in `.rodata` on both sides, so the store is fatal in C too. Tested in a forked child on each side; both must die with the same signal (SIGSEGV/11) | [x] |

## Boundary cases covered even though the C has no explicit check

- zero (`0`), one (`1`), minus one (`-1`) for every `int` parameter
- `INT_MIN`, `INT_MIN + 1`, `INT_MAX - 1`, `INT_MAX`
- one step past the `switch` range on both sides (`-1` and `7`)
- `atoi` inputs: empty, whitespace-only, sign-only (`"-"`, `"+"`), digits after
  non-digits, `LONG_MIN`/`LONG_MAX` ± 1, 40-digit numbers
- there is no length/size/pointer parameter anywhere in the API, so the
  "null pointer / zero length / oversized length" class collapses to row 17
