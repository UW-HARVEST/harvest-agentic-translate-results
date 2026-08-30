# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation

`c_src/include/driver.h` exposes exactly one entry point, which is also the
lowest-level one — there is no convenience wrapper / one-shot layer to hide
behind:

```c
void driver(char c);
```

**Runtime options / modes / flags:** the public API has none (no setter, no
flags word, no `#ifdef` in the library). The *one* piece of global state the
function touches is the C locale, which it forces itself:
`setlocale(LC_ALL, "C")` at `c_src/src/driver.c:32`. That is an axis, because
the caller can put the process in a different locale beforehand and the reset
must neutralise it (rows C18–C20).

**Input shapes the code special-cases:** `driver` has a single 1-byte argument
and no explicit branches, so every branch it takes is inside the glibc
`<ctype.h>` table lookups it feeds `c` into. The distinct shapes are therefore
the equivalence classes of the `__ctype_b` / `__ctype_tolower` / `__ctype_toupper`
tables — i.e. each distinct *combination of the 12 class bits* that the 14
`printf`s can reveal — plus the sign behaviour of `char` (glibc's tables are
indexed `-128..=255`, and a signed `char` reaches the negative half), plus the
ABI shape of the argument itself (a `char` parameter accepts any `int` at the
call boundary).

Cross-product of {class equivalence class} × {sign of `char`} × {argument
passed as `c_char` / as a wide `c_int`} × {fresh call / repeated call / locale
perturbed}, pruned to the combinations the tables actually distinguish:

| #   | entry point(s) | configuration (options set + input shape) | [ ] |
|-----|----------------|--------------------------------------------|-----|
| C1  | `driver` | locale reset by callee; `c` = NUL `0x00` (cntrl, not space) | [x] |
| C2  | `driver` | `c` in `0x01..=0x08` — pure `_IScntrl`, non-space (randomized over the range) | [x] |
| C3  | `driver` | `c` = `0x09` `\t` — cntrl + space + **blank** | [x] |
| C4  | `driver` | `c` in `0x0A..=0x0D` (`\n \v \f \r`) — cntrl + space, **not** blank | [x] |
| C5  | `driver` | `c` in `0x0E..=0x1F` — pure cntrl (randomized) | [x] |
| C6  | `driver` | `c` = `0x20` space — print + space + blank, **not** graph, **not** cntrl | [x] |
| C7  | `driver` | `c` in `0x21..=0x2F` — punct/graph/print (randomized) | [x] |
| C8  | `driver` | `c` in `'0'..='9'` — digit + xdigit + alnum, **not** alpha (randomized) | [x] |
| C9  | `driver` | `c` in `0x3A..=0x40` — punct (randomized) | [x] |
| C10 | `driver` | `c` in `'A'..='F'` — upper + alpha + alnum + **xdigit** (randomized) | [x] |
| C11 | `driver` | `c` in `'G'..='Z'` — upper + alpha + alnum, **not** xdigit (randomized) | [x] |
| C12 | `driver` | `c` in `0x5B..=0x60` — punct incl. `` ` `` boundary just below `'a'` (randomized) | [x] |
| C13 | `driver` | `c` in `'a'..='f'` — lower + alpha + alnum + **xdigit** (randomized) | [x] |
| C14 | `driver` | `c` in `'g'..='z'` — lower + alpha + alnum, **not** xdigit (randomized) | [x] |
| C15 | `driver` | `c` in `0x7B..=0x7E` — punct above `'z'` (randomized) | [x] |
| C16 | `driver` | `c` = `0x7F` DEL — cntrl, **not** print/graph; last positive table slot | [x] |
| C17 | `driver` | `c` in `0x80..=0xFF` → **negative** `char` (`-128..=-1`): negative table index, all classes `0`, case-mapping identity, `%c` re-widens to the original byte (randomized) | [x] |
| C18 | `driver` | **exhaustive**: every one of the 256 `char` bit patterns, one call each | [x] |
| C19 | `driver` | repeated calls with the same `c` (idempotence / no accumulated state) | [x] |
| C20 | `driver` | caller sets a non-`"C"` locale (`en_US.UTF-8`, `C.UTF-8`, `POSIX`) first, then calls — callee's `setlocale(LC_ALL,"C")` must neutralise it | [x] |
| C21 | `driver` | interleaved C/Rust calls in one process, random `c` sequence (shared libc stdio stream, no cross-contamination) | [x] |
| C22 | `driver` | argument passed across FFI as a **wide `c_int`** with garbage in bits 8..31 (low byte still decides), randomized | [x] |
| C23 | `driver` | randomized whole-alphabet sweep: 4000 random `c` values, fixed seed, byte-for-byte stream compare | [x] |

Every row is exercised against **both** `.so`s loaded via `libloading` and the
14 printed lines compared byte-for-byte (including embedded NUL bytes).

## Findings

**Divergence found and fixed (rows C22 / E6 / E7).** In the **release** build only,
`driver` **segfaulted** when a caller passed an `int` that does not fit in a
`char` (e.g. `0x1234_5641`), while the C printed the results for the low byte
(`'A'`). Cause: `src/ctype.rs` guarded its 384-entry table with
`if c >= -128 && c < 256`, but the argument arrives as an `i8`, which the ABI
declares sign-extended, so the optimiser was entitled to fold the guard away and
index with the full-width register — an out-of-bounds read. The debug build
happened to keep the guard, which is why the bug was invisible at `-O0` and only
the release artifact crashed.

Fix: the ctype tables are now keyed by the `char`'s byte (`[c_int; 256]`), so
every lookup is structurally in bounds at any optimisation level, and the
resulting truncation reproduces the C's behaviour exactly. Observationally
identical across the whole `-128 ..= 255` range, since in the `"C"` locale both
the negative half and the `128 ..= 255` half of glibc's table are empty.

This is exactly the class of bug the "run every row under every profile" rule
exists to catch: rows C1–C21 passed identically in both profiles.
