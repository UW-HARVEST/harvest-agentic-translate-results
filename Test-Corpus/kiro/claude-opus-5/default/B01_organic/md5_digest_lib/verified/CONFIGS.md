# CONFIGS.md — Configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Axes derived mechanically from the
C source, then crossed and pruned to the combinations the C actually
distinguishes.

## Axis enumeration (from the source, not from guesses)

**Public entry points** — the full set, from `include/lib.h`:

| entry point | kind |
|---|---|
| `md5_digest(const tflac_md5*, tflac_u8[16])` | the **only** symbol, and it is the lowest-level one — there is no convenience/one-shot wrapper layered over anything, so "test the low-level API too" is satisfied by testing it directly |

**Runtime options / modes / flags:** none. `grep -nE 'if\|switch\|#if'` over the
library returns no hits, so there is no flag the API can set and no branch to
toggle. No option axis exists.

**Input shapes the code special-cases:** the signature is fully fixed — one
16-byte struct in, one 16-byte buffer out. There is no count, no width, no
element type, no format selector, and no empty/one/many axis. What *is*
value-dependent is the 16 independent byte-extraction expressions
`(tflac_u8)(field >> {0,8,16,24})` for `field ∈ {a,b,c,d}`. So the shapes that
the code can distinguish are:

- **F — field identity** (which of the 4 words a byte came from): struct offsets 0/4/8/12
- **P — byte position within a word** (which shift): 0/8/16/24
- **V — value pattern**: zero, all-ones, single-bit, high-bit-set, per-byte-distinct, random
- **A — `out` pointer alignment**: 4-aligned vs offset 1/2/3 (legal: `out` is `uint8_t*`)
- **M — `m` pointer alignment**: 4-aligned (natural) vs misaligned
- **O — `m`/`out` overlap**: disjoint, or aliased at various offsets. **Neither
  parameter is `restrict`-qualified**, so overlap is a legal call. The C reads
  `m->a`, `m->b`, `m->c`, `m->d` *interleaved with* the stores, so with overlap
  the later reads observe bytes the earlier stores already wrote. This is the
  one place where a naive Rust translation can legitimately diverge (a Rust
  `&tflac_md5` shared reference permits LLVM to hoist all four loads above the
  stores), so it is enumerated as a first-class axis rather than skipped.
- **B — buffer extent**: exactly 16 bytes, larger than 16 (must leave the tail
  untouched), and 16 bytes ending flush against an unmapped guard page (must
  not read or write a 17th byte)
- **S — call sequencing**: single call, repeated calls into one buffer, one
  struct into two buffers (must be stateless)

Endianness: the shift expressions hard-code little-endian output independent of
host byte order; the test host is x86-64 LE. Both implementations are
byte-order-explicit (`>> 8*k` vs `to_le_bytes`), so there is no host-dependent
row that can be exercised here; noted for completeness.

## Rows (cross-product, pruned to what the C distinguishes)

Every row is driven with **many randomized inputs (fixed seed 0x5EED_1234)**
in addition to the named pattern, and asserts the 16 output bytes from the C
`.so` and the Rust `.so` are byte-identical.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `md5_digest` | V=all zero: `a=b=c=d=0x00000000`; A=aligned, O=disjoint, B=exact 16 | [x] |
| C2 | `md5_digest` | V=all ones: `a=b=c=d=0xFFFFFFFF` | [x] |
| C3 | `md5_digest` | F=isolate `a`: `a=0xDEADBEEF`, `b=c=d=0` — pins struct offset 0 → out[0..4] | [x] |
| C4 | `md5_digest` | F=isolate `b`: `b=0xDEADBEEF`, others 0 — pins offset 4 → out[4..8] | [x] |
| C5 | `md5_digest` | F=isolate `c`: `c=0xDEADBEEF`, others 0 — pins offset 8 → out[8..12] | [x] |
| C6 | `md5_digest` | F=isolate `d`: `d=0xDEADBEEF`, others 0 — pins offset 12 → out[12..16] | [x] |
| C7 | `md5_digest` | F×P full permutation: per-byte-distinct words `0x04030201/0x08070605/0x0C0B0A09/0x100F0E0D` — every one of the 16 output bytes gets a unique value, so any field-order **or** shift-order transposition is detected | [x] |
| C8 | `md5_digest` | P=high bit / sign-sensitive: `0x80808080`, `0x80000000`, `0x00000080`, `0x7F7F7F7F` — catches signed-shift or sign-extension mistakes | [x] |
| C9 | `md5_digest` | V=single-bit sweep: all **128** inputs with exactly one of the 128 struct bits set (exhaustive over bit positions) | [x] |
| C10 | `md5_digest` | V=random, full 32-bit range on all 4 fields, 20 000 seeded iterations | [x] |
| C11 | `md5_digest` | A=misaligned `out` at byte offsets 1, 2, 3 of a heap buffer (× random values) | [x] |
| C12 | `md5_digest` | M=misaligned `m` at byte offsets 1, 2, 3 (struct bytes copied in, pointer cast) — Rust must not assume 4-alignment | [x] |
| C13 | `md5_digest` | B=oversized `out` (64 bytes, 0xAA-filled): bytes 16..63 must remain 0xAA in both | [x] |
| C14 | `md5_digest` | B=`out` occupying the final 16 bytes before an unmapped guard page — proves no 17th-byte touch, in both | [x] |
| C15 | `md5_digest` | O=full alias: `out == (tflac_u8*)m` (writes land on the words being read) | [x] |
| C16 | `md5_digest` | O=alias at `out = (tflac_u8*)m + 1` (unaligned overlap) | [x] |
| C17 | `md5_digest` | O=alias at `out = (tflac_u8*)m + 4` (out[0..4] overwrites `b` before it is read) | [x] |
| C18 | `md5_digest` | O=alias at `out = (tflac_u8*)m + 8` and `+12` (later fields clobbered mid-call) | [x] |
| C19 | `md5_digest` | O=reverse alias: `out = (tflac_u8*)m - 8`, so `out`'s tail overlaps `a`/`b` | [x] |
| C20 | `md5_digest` | O=partial alias × V=random, 2 000 seeded iterations over overlap offsets −16..+16 | [x] |
| C21 | `md5_digest` | S=repeated calls: same `out` buffer written 3× (2 different structs then the first again) — no residual state | [x] |
| C22 | `md5_digest` | S=stateless: one struct → two separate buffers, and C-then-Rust vs Rust-then-C ordering both agree | [x] |

## No binary executable

`c_src/CMakeLists.txt` contains a single `add_library(... SHARED src/lib.c)` and
no `add_executable`, and `translation/Cargo.toml` declares
`crate-type = ["cdylib"]` with no `[[bin]]` and no `src/main.rs`. There is no
driver program, so the "compare C and Rust stdout byte-for-byte" requirement is
vacuous for this project.

## Feature combinations

No `[features]` in `Cargo.toml` (`grep -c '\[features\]'` → 0), so the default
build is the only build. Phase D re-runs the whole suite under
`--no-default-features` to confirm.

## Results

All 22 rows pass against both the debug and release Rust `.so`, under both
`cargo test` and `cargo test --no-default-features` (29 tests × 4
configurations). Two real divergences were found and fixed in the Rust; the C
was not touched.

### Divergence 1 — row C12, misaligned `m` (found by C12)

The Rust formed a `&tflac_md5` shared reference from the raw pointer
(`let m = unsafe { &*m };`). With the struct placed at byte offset 1 of a heap
buffer, that aborted:

```
misaligned pointer dereference: address must be a multiple of 0x4 but is 0x7f83ec0010b1
```

whereas the C performs an ordinary (unaligned-capable) x86-64 dword load and
returns the correct bytes. Fixed by removing the reference and the
`&mut [u8]` slice entirely and working through raw pointers at byte
granularity, where alignment is vacuous.

### Divergence 2 — rows C15–C20, read/store interleaving under aliasing

Neither C parameter is `restrict`-qualified, so `out` may overlap `*m`.
Disassembling the actual C `.so` (`CMAKE_BUILD_TYPE` is empty, so no
optimization) shows the field is **reloaded from memory before each of the 16
single-byte stores**:

```
mov -0x8(%rbp),%rax ; mov (%rax),%eax ; mov %dl,(%rax)     ; out[0] = (u8)a
mov -0x8(%rbp),%rax ; mov (%rax),%eax ; shr $0x8,%eax ; ... ; out[1] = (u8)(a>>8)
```

The Rust instead read each field once and wrote 4 bytes at a time via
`copy_from_slice` (a `memcpy`), so with overlap it produced the *pre-call*
field values where the C produces the progressively-overwritten ones. Fixed by
reading the source byte freshly inside every iteration and emitting exactly one
single-byte store per step.

`cfg_aliasing_rows_are_discriminating` proves these rows are not vacuous: it
builds a model of the hoisted-load implementation and asserts the real C
disagrees with it in at least 4 of 6 overlap offsets, and that with
`out = m + 1` the C produces the byte-propagation cascade
(`out[17..33] == [out[16]; 16]`) that only per-byte reloading can produce. Any
implementation that hoisted its loads would therefore be caught here.

### Equivalence note

Because `shift == 8*(i%4)` selects byte `i%4` of the field at offset `4*(i/4)`,
the byte the C's shift extracts always lives at absolute offset
`4*(i/4) + (i%4) == i`. Reading only that byte observes the same memory state
as the C's full dword load followed by the shift, so the final Rust is a
straight sequential byte copy — which is exactly what the C computes, aliasing
included.
