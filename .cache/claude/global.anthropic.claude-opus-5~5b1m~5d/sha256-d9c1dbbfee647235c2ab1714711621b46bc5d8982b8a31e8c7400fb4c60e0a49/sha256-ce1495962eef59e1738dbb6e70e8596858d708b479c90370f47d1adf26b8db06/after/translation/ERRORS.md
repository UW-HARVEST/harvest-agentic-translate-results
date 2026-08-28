# ERRORS.md — Error / rejection surface table (Phase A, gate for Phase C)

Mechanically derived from `c_src/src/lib.c`. Every `return`, every branch, every
sentinel is accounted for below — nothing invented, nothing from docs.

## Mechanical extraction

The whole of `c_src/src/lib.c` (the only translation unit):

```c
char *custom_strdup(const char *str)
{
  size_t len;
  char *newstr;

  if(!str)                 /* <-- rejection #1 */
    return (char *)NULL;

  len = strlen(str) + 1;   /* size_t wraparound is the only arithmetic */

  newstr = malloc(len);
  if(!newstr)              /* <-- rejection #2 */
    return (char *)NULL;

  memcpy(newstr, str, len);
  return newstr;           /* success */
}
```

Grep inventory of every rejection construct in `c_src/`:

| construct | occurrences | where |
|-----------|-------------|-------|
| `return (char *)NULL` | 2 | the two rows below |
| `return -1` / error codes / error enums | 0 | *(none — the API's only failure channel is a `NULL` return)* |
| `RETURN_ERROR`-style macro | 0 | none defined or used |
| `assert` / `NDEBUG` | 0 | none |
| `errno` set or read | 0 | none — the C never touches `errno` |
| explicit range / min / max check | 0 | no `#define` limits, no size clamps, no bounds compare |
| null check | 2 | `if(!str)`, `if(!newstr)` |
| enum parameters | 0 | the API takes no enum / flag / mode argument |

There are exactly **2** distinct rejection paths.

## Error-surface table

| # | function | trigger (exact invalid input/condition) | expected C result | test | status |
|---|----------|------------------------------------------|-------------------|------|--------|
| E1 | `custom_strdup` | `str == NULL` — the argument is a null pointer, so `if(!str)` is taken | returns `NULL` (`(char *)0`); no allocation performed; `errno` untouched | `tests/error_paths.rs::e1_null_pointer_input` | [x] |
| E2 | `custom_strdup` | `malloc(strlen(str)+1)` returns `NULL` — allocator exhaustion, so `if(!newstr)` is taken. Reproduced deterministically by lowering `RLIMIT_AS` to the process's current address-space size before calling with an 8 MiB string, which forces glibc's `mmap` path to fail | returns `NULL`; the already-computed `len` is discarded; **no `memcpy` is performed** (i.e. no crash, no partial write) | `tests/malloc_failure.rs::e2_malloc_returns_null` | [x] |

### Notes on E1

`!str` is a plain pointer-nullness test, so the *only* rejected pointer value is
`(char *)0`. Any other value — including a wild/unmapped non-null pointer — is
**not** rejected by the C and is undefined behaviour to pass; such pointers are
therefore deliberately excluded from the differential tests (they would crash
both implementations identically but untestably).

### Notes on E2

E2 is the only path where the two implementations could plausibly diverge: a
Rust translation that used the Rust global allocator (`alloc::alloc`) instead of
`malloc` would **abort** the process on allocation failure instead of returning
`NULL`. The translation calls libc `malloc` directly, so it returns `NULL` like
the C. The test asserts both return `NULL` *and* that the process survives.

## Generic FFI boundary cases required by Phase C

These are covered even though the C has no explicit check for them, because
Phase C mandates the generic boundaries every C API has:

| # | condition | expected behaviour (C, and therefore Rust) | test | status |
|---|-----------|--------------------------------------------|------|--------|
| G1 | null pointer | see E1 — returns `NULL` | `e1_null_pointer_input` | [x] |
| G2 | zero length input: `""` (valid, not an error) | `len == 1`; returns a 1-byte allocation holding just `'\0'`; **non-NULL** | `g2_empty_string_is_not_an_error` | [x] |
| G3 | out-of-range enum value across FFI | **not applicable, and proven so**: `custom_strdup` has exactly one parameter of type `const char *` and no enum/flag/mode parameter anywhere in `lib.h`, so there is no enum whose value could fall outside its variants. The nearest analogue — a `char` payload byte outside the "expected" ASCII range — is covered as a *valid* input by `CONFIGS.md` rows C6/C7 (all 255 non-NUL byte values, incl. `0x80..=0xFF` which are negative in a signed `char`) | `g3_no_enum_parameters_all_byte_values_are_valid` | [x] |
| G4 | oversized length | The C imposes **no** maximum length (no range check exists to violate). "Oversized" therefore degenerates into E2: the length is only rejected when `malloc` cannot satisfy it. Large-but-satisfiable lengths (1 MiB, 8 MiB) are valid inputs and are covered by `CONFIGS.md` row C8 | `e2_malloc_returns_null`, `CONFIGS` C8 | [x] |
| G5 | one step past a documented valid range | There is no documented valid range and no constant to step past (0 range checks, 0 min/max constants in the source). The only boundary in the code is the `strlen`/NUL boundary itself, which is probed by C9 (NUL as the final byte before an unmapped page) | `CONFIGS` C9 | [x] |
| G6 | repeated / interleaved failure and success calls | each call is independent and stateless (no globals, no statics in `lib.c`); a `NULL` return must not poison later calls | `g6_failure_does_not_poison_later_calls` | [x] |

## Phase C gate

All rows above are checked. **E1, E2, G1–G6 have passing differential tests
against both `.so`s.**
