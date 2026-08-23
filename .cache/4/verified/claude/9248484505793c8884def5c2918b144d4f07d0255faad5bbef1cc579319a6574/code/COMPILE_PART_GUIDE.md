# How to translate a part of pcre2_compile.c

`c_src/src/pcre2_compile.c` is translated into several Rust files that are all
`include!`d by `src/pcre2_compile.rs`, so they all end up in ONE Rust module:
every function/static in any part can be used by any other part, exactly as in
the C file.

Read, in this order:
1. `TRANSLATION_GUIDE.md` (general conventions - obey it)
2. `src/internal.rs` (types, structs, constants, callable library functions)
3. `src/macros.rs` (Rust versions of the C macros)
4. `src/pcre2_compile.rs` (the include! chain, so you can see which part is which)
5. `c_src/src/pcre2_compile.h` (META_* codes, ERR* codes, PC_* indices, the
   `eclass_op_info` struct, NAMED_GROUP_* macros)
6. Your region of `c_src/src/pcre2_compile.c`.

## Shape of your output file

ONLY item definitions (functions, statics, consts, structs) - no `use`
statements, no `mod`, and nothing that is already declared elsewhere:

```rust
/* Translated from c_src/src/pcre2_compile.c lines NNN-MMM */

static ...;

unsafe fn read_number(...) -> BOOL { ... }
```

* C `static` functions -> plain private `unsafe fn` (no `pub`, no `no_mangle`).
* C functions declared with `PRIV(name)` or `PCRE2_EXP_DEFN` are exported: use
  `#[unsafe(no_mangle)] pub unsafe extern "C" fn <linker name>(...)` with the
  linker name given in your task description (they are also declared in
  `src/internal.rs`, so a signature mismatch is a compile error).
* C `static const` tables -> private `static NAME: [T; N] = [...]` with exactly
  the same values (compute macro values such as `IMM2_SIZE` = 2, `LINK_SIZE` = 2,
  `MAX_REPEAT_COUNT` = 65535 by hand).
* Do NOT define `_pcre2_posix_class_maps8` - it is already defined in
  `src/pcre2_compile_class.rs`. Where the C code uses `PRIV(posix_class_maps)`,
  refer to `_pcre2_posix_class_maps8` (it is re-exported by `internal.rs`).
* Do NOT translate `#ifdef DEBUG_SHOW_PARSED` / `DEBUG_SHOW_OPS` /
  `#ifdef SUPPORT_JIT` / EBCDIC / 16-bit / 32-bit code.
* `PCRE2_ASSERT(x)`, `PCRE2_DEBUG_UNREACHABLE()`, `PCRE2_UNREACHABLE()` are
  no-ops in this build: translate them as nothing (a comment). NEVER a panic.

## Translating `goto`

Most gotos in this file are forward jumps to an error handler or to shared code
later in the same function. Use nested labelled blocks, with the block for the
EARLIEST label innermost:

```rust
// C:
//   ... goto FAILED; ...
//   ... goto ESCAPE_FAILED; ...
//   ESCAPE_FAILED: ... falls through to FAILED
//   FAILED: return -1;
'failed: {
    'escape_failed: {
        ...body...  // `goto FAILED` => break 'failed;  `goto ESCAPE_FAILED` => break 'escape_failed;
        /* the normal (non-goto) exit path of the body */
        return ...;
    }
    /* ESCAPE_FAILED: code */
    ...
    /* falls through into FAILED */
}
/* FAILED: code */
...
```

Rules:
* `goto L` -> `break 'l;` where the block labelled `'l` is closed exactly where
  the C label `L:` appears.
* A backward `goto` (a loop) -> `'l: loop { ... continue 'l; ... break; }`.
* Keep the C variable names, so the code stays comparable with the original.

## Types and idioms

* `BOOL` is `c_int` (`TRUE`/`FALSE`); `if (x)` for an int -> `if x != 0`.
* `PCRE2_SPTR` = `*const u8`, `PCRE2_UCHAR` = `u8`, `PCRE2_SIZE` = `usize`.
* Pointer-to-pointer parameters: `PCRE2_SPTR *ptrptr` -> `*mut PCRE2_SPTR`,
  read/write with `*ptrptr`.
* `*ptr++` -> `{ let t = *ptr; ptr = ptr.add(1); t }`; `p - q` ->
  `p.offset_from(q)` (an isize - cast as C does).
* The parsed-pattern vector is `*mut u32`; use `META_CODE!(x)`, `META_DATA!(x)`,
  `META_DIFF!(x,y)`, `PUTOFFSET!(s,p)`, `GETOFFSET!(s,p)`,
  `GETPLUSOFFSET!(s,p)`, `READPLUSOFFSET!(s,p)`, `SKIPOFFSET!(p)`, `SIZEOFFSET`.
* Compiled-code output uses `PUT!`, `PUT2!`, `PUTINC!`, `PUT2INC!`, `GET!`,
  `GET2!` and `_pcre2_OP_lengths_8`.
* Character constants are `CHAR_x` (u32): compare a code unit as
  `*p as u32 == CHAR_a`.
* String literals: `b"..."` byte strings. The C `STR_x` macros are single ASCII
  characters (`STR_a` = "a", `STR_LEFT_PARENTHESIS` = "(", `STR_0` = "0", ...),
  and `STRING_xxx0` macros are the obvious NUL-terminated words (e.g.
  `STRING_ACCEPT0` = `b"ACCEPT\0"`). Concatenated string tables must be
  reproduced **byte for byte** because the code indexes into them.
* Tables must be read through `*TABLE.as_ptr().add(i)` when the index might be
  out of range (never a bounds-check panic).
* Where C takes the address of a local array (`uint32_t stack[...]`), use
  `let mut stack: [u32; N] = [0; N];` and `stack.as_mut_ptr()`.
* `memcpy`/`memmove`/`memset`/`malloc`/`free`/`memcmp`/`strlen` are declared in
  `internal.rs`; call them exactly where C does.

## Verification

```
cd $HARVEST_WORKDIR/translated_rust && cargo build --release 2>&1 | grep -E "^error" -A15 | head -80
```
Other agents are writing the other parts at the same time, and some parts are
still placeholders, so you will see errors about missing functions from other
parts (e.g. `cannot find function parse_regex`). **Only fix errors whose span
points at YOUR file.** Iterate until your file is clean. Never edit another
agent's file.
