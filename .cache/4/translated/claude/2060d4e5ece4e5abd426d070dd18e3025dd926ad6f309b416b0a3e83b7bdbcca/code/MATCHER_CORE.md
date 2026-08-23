# Translating the `match()` function of `pcre2_match.c`

Read `TRANSLATION_GUIDE.md` first, then this file, then `src/matcher_core.rs`.

The C function `match()` (C lines 684–6950 of `c_src/src/pcre2_match.c`) is one
enormous function built around `goto`. It has been reorganised in
`src/matcher_core.rs` into a single flat dispatch loop:

```rust
let mut lbl: u32 = LBL_NEW_FRAME;
'sw: loop {
    if lbl == LBL_MATCH_RECURSE { /* C lines 753-846 */  lbl = LBL_NEW_FRAME; }
    if lbl == LBL_NEW_FRAME     { /* C lines 849-877 */  lbl = LBL_TOP_OF_LOOP; }
    if lbl == LBL_TOP_OF_LOOP   { (*F).op = *(*F).ecode; lbl = LBL_SWITCH; }
    if lbl == LBL_RETURN_SWITCH { /* C lines 6909-6944 */ lbl = LBL_RM_BASE + return_id; }

    include!("matcher_arms/a.rs");   // C  900-1659
    include!("matcher_arms/b.rs");   // C 1660-2050
    include!("matcher_arms/c.rs");   // C 2051-2573
    include!("matcher_arms/d.rs");   // C 2574-2918
    include!("matcher_arms/e.rs");   // C 2919-3775
    include!("matcher_arms/e2.rs");  // C 3776-4368
    include!("matcher_arms/e3.rs");  // C 4369-5248
    include!("matcher_arms/f.rs");   // C 5249-5488
    include!("matcher_arms/g.rs");   // C 5489-5988
    include!("matcher_arms/h.rs");   // C 5989-6569
    include!("matcher_arms/i.rs");   // C 6570-6890

    return PCRE2_ERROR_INTERNAL;     // the switch's `default:`
}
```

`include!` requires each chunk file to be **exactly one block expression**, i.e.
the whole file must be:

```rust
{
/* ... your statements ... */
}
```

(a single outer `{` on the first line and a single matching `}` on the last).
Do NOT put `use` statements, `fn` items, `static` items or `const` items in a
chunk file — items are not allowed to be defined in these files because they
would be re-defined... actually they ARE allowed inside the block (block-scoped
items), so `static` lookup tables needed by only your chunk may be declared
inside your block. Prefer that over touching other files.

## The mandatory shape of a chunk file

```rust
{
if lbl == LBL_SWITCH {
    match (*F).op as u32 {
        OP_CLOSE => {
            /* ... translated body of `case OP_CLOSE:` ... */
            lbl = LBL_TOP_OF_LOOP; continue 'sw;   /* == C `break` */
        }
        OP_ANY | OP_ALLANY => {
            ...
        }
        _ => {}     /* not one of my opcodes: fall through to the next chunk */
    }
}

/* Intra-switch labels that other chunks jump to, if any belong to me: */
if lbl == LBL_REPEATCHAR {
    ...
}

/* RMATCH continuation points that belong to me: */
if lbl == LBL_RM_BASE + RM1 as u32 {
    ...
}
}
```

**Every path out of a handled opcode/label must `continue 'sw`, `return`, or
`break` out of a construct you created yourself. Never fall out of the bottom of
an `if lbl == ...` block after handling something**, because that would fall into
the next chunk.

## Statement translation table

| C | Rust |
|---|---|
| `break;` (ending a `case` of the opcode switch) | `lbl = LBL_TOP_OF_LOOP; continue 'sw;` |
| `continue;` (continue the main `for(;;)` loop)  | `lbl = LBL_TOP_OF_LOOP; continue 'sw;` |
| `RMATCH(ra, rb);`                               | `start_ecode = ra; (*F).return_id = rb; lbl = LBL_MATCH_RECURSE; continue 'sw;` and the code that follows the RMATCH goes into an `if lbl == LBL_RM_BASE + rb as u32 { ... }` block in your chunk |
| `RRETURN(ra);`                                  | `rrc = ra; lbl = LBL_RETURN_SWITCH; continue 'sw;` |
| `goto XXX;` where `XXX:` is in **another** chunk | `lbl = LBL_XXX; continue 'sw;` |
| `goto XXX;` where `XXX:` is inside **your own** chunk | use a Rust labelled block: wrap the region from the `goto` up to the label in `'xxx: { ... break 'xxx; ... }` so that the code *after* the label still runs in the right enclosing scope. (Do NOT use the `lbl` mechanism for labels that are nested inside blocks, because that loses the enclosing block context.) |
| `return e;`                                     | `return e;` |
| `PCRE2_ASSERT(x)` / `PCRE2_DEBUG_UNREACHABLE()` / `PCRE2_UNREACHABLE()` / `PCRE2_FALLTHROUGH` | nothing |

Careful: a `break` inside a `for`/`while`/`do..while`/inner `switch` **inside** a
case body breaks that inner construct, not the opcode switch — translate those as
Rust `break` of the corresponding Rust loop, not as `lbl = LBL_TOP_OF_LOOP`.

Careful: a C `continue` inside a `for`/`while` loop inside a case body continues
that loop, not the main loop.

## Cross-chunk labels (use the `lbl` mechanism for these only)

| C label | line | constant | lives in chunk |
|---|---|---|---|
| `REPEATCHAR`    | 1392 | `LBL_REPEATCHAR` | a |
| `REPEATNOTCHAR` | 1733 | `LBL_REPEATNOTCHAR` | b |
| `REPEATTYPE`    | 2973 | `LBL_REPEATTYPE` | e |
| (split point, C 3776) | 3776 | `LBL_REPEATTYPE_2` | e2 |
| (split point, C 4369) | 4369 | `LBL_REPEATTYPE_3` | e3 |
| `REF_REPEAT`    | 5278 | `LBL_REF_REPEAT` | f |
| `POSSESSIVE_NON_CAPTURE` | 5545 | `LBL_POSSESSIVE_NON_CAPTURE` | g |
| `POSSESSIVE_CAPTURE`     | 5553 | `LBL_POSSESSIVE_CAPTURE` | g |
| `POSSESSIVE_GROUP`       | 5557 | `LBL_POSSESSIVE_GROUP` | g |
| `GROUPLOOP`              | 5676 | `LBL_GROUPLOOP` | g |
| `ASSERT_NOT_FAILED`      | 5853 | `LBL_ASSERT_NOT_FAILED` | g |
| `SCS_OFFSET_FOUND`       | 5907 | `LBL_SCS_OFFSET_FOUND` | g |
| `ASSERT_NL_OR_EOS`       | 6604 | `LBL_ASSERT_NL_OR_EOS` | i |

All the `goto`s for `REPEATCHAR`, `REPEATNOTCHAR`, `REF_REPEAT`,
`POSSESSIVE_*`, `GROUPLOOP`, `ASSERT_NOT_FAILED`, `SCS_OFFSET_FOUND` and
`ASSERT_NL_OR_EOS` happen inside the same chunk as their label, so you may
instead use a plain Rust labelled block if that is cleaner — EXCEPT that other
chunks must still be able to reach a label if a `goto` for it exists outside your
range. The only genuinely cross-chunk jumps are:
* `goto REPEATTYPE` at C 2922/2930/2937/2944/2951/2958 (chunk e) → label in
  chunk e (same chunk, so a labelled block also works)
* the two artificial split points `LBL_REPEATTYPE_2` (chunk e → e2) and
  `LBL_REPEATTYPE_3` (chunk e2 → e3).

## Field-name mapping (the `Fxxx` macros)

| C macro | Rust |
|---|---|
| `Fback_frame` | `(*F).back_frame` |
| `Fcapture_last` | `(*F).capture_last` |
| `Fcurrent_recurse` | `(*F).current_recurse` |
| `Fecode` | `(*F).ecode` |
| `Feptr` | `(*F).eptr` |
| `Fgroup_frame_type` | `(*F).group_frame_type` |
| `Flast_group_offset` | `(*F).last_group_offset` |
| `Fmark` | `(*F).mark` |
| `Frdepth` | `(*F).rdepth` |
| `Fstart_match` | `(*F).start_match` |
| `Foffset_top` | `(*F).offset_top` |
| `Fop` | `(*F).op` (a `u8`; compare with `as u32`) |
| `Fovector[k]` | `*ovec(F).add(k)`  (`ovec()` is in `matcher_core.rs`) |
| `Freturn_id` | `(*F).return_id` (a `u8`) |
| `N->ovector[k]` | `*ovec(N).add(k)` |
| `P->ovector[k]` | `*ovec(P).add(k)` |

The `Lxxx` macros that are `#define`d at the point of use inside a case body map
onto the `fields` union of `heapframe`. E.g.

```
#define Lstart_eptr  F->fields.char_repeat.start_eptr   ->  (*F).fields.char_repeat.start_eptr
#define Lcharptr     F->fields.char_repeat.charptr      ->  (*F).fields.char_repeat.charptr
#define Lmin         F->fields.char_repeat.min          ->  (*F).fields.char_repeat.min
#define Lmax         F->fields.char_repeat.max          ->  (*F).fields.char_repeat.max
#define Lc           F->fields.char_repeat.c            ->  (*F).fields.char_repeat.c
#define Loc          F->fields.char_repeat.oc.oc        ->  (*F).fields.char_repeat.oc.oc
#define Locchars     F->fields.char_repeat.oc.occu      ->  (*F).fields.char_repeat.oc.occu
```
Look at the actual `#define`s in the C right above each case body and translate
them to the corresponding `(*F).fields.<variant>.<member>` path. The
`heapframe`/`hf_fields` definitions are in `src/internal.rs`; the union variant
struct names are `hf_char_repeat`, `hf_charnot_repeat`, `hf_class_repeat`,
`hf_xclass_repeat`, `hf_eclass_repeat`, `hf_type_repeat`, `hf_ref_repeat`,
`hf_op_bra`, `hf_op_brapos`, `hf_op_recurse`, `hf_op_assert_scs`, `hf_op_cond`,
`hf_op_vreverse` and the union members are named `char_repeat`,
`charnot_repeat`, `class_repeat`, `xclass_repeat`, `eclass_repeat`,
`type_repeat`, `ref_repeat`, `op_bra`, `op_brapos`, `op_recurse`,
`op_assert_scs`, `op_cond`, `op_vreverse` (all `unsafe` union field accesses).
`occu` is `[u8; 4]`; index it with `(*F).fields.char_repeat.oc.occu[k]`.

## In-scope local variables (declared in `match_()`)

`F`, `N`, `P` (`*mut heapframe`), `frames_top`, `assert_accept_frame`,
`frame_copy_size`, `frame_size`, `match_data`, `mb`, `top_bracket`,
`start_ecode`, `branch_end`, `branch_start`, `bracode` (`PCRE2_SPTR`),
`offset`, `length` (`PCRE2_SIZE`), `rrc`, `proptype` (`c_int`),
`i`, `fc`, `number`, `reptype`, `group_frame_type` (`u32`),
`condition`, `cur_is_word`, `prev_is_word`, `utf`, `ucp` (`BOOL` == `c_int`),
`lbl` (`u32`).

`mb->xxx` -> `(*mb).xxx`; `match_data->xxx` -> `(*match_data).xxx`.
Pointer/frame helpers: `frame_add(f, bytes)`, `frame_sub(f, bytes)`,
`frame_at(base, byte_offset)`.
`(heapframe *)((char *)match_data->heapframes + offset)` ->
`frame_at((*match_data).heapframes, offset)`.

## Macros used inside the case bodies

```
CHECK_PARTIAL():
    if (*F).eptr >= (*mb).end_subject { /* SCHECK_PARTIAL */
        if (*mb).partial != 0 && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE) {
            (*mb).hitend = TRUE;
            if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
        }
    }

SCHECK_PARTIAL():
    if (*mb).partial != 0 && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE) {
        (*mb).hitend = TRUE;
        if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
    }

IS_NEWLINE(p):
    ( if (*mb).nltype != NLTYPE_FIXED {
          p < (*mb).end_subject &&
          crate::newline::_pcre2_is_newline_8(p, (*mb).nltype, (*mb).end_subject,
              &mut (*mb).nllen, utf) != FALSE
      } else {
          p <= (*mb).end_subject.sub((*mb).nllen as usize) &&
          *p as u32 == (*mb).nl[0] as u32 &&
          ((*mb).nllen == 1 || *p.add(1) as u32 == (*mb).nl[1] as u32)
      } )

WAS_NEWLINE(p):
    ( if (*mb).nltype != NLTYPE_FIXED {
          p > (*mb).start_subject &&
          crate::newline::_pcre2_was_newline_8(p, (*mb).nltype, (*mb).start_subject,
              &mut (*mb).nllen, utf) != FALSE
      } else {
          p >= (*mb).start_subject.add((*mb).nllen as usize) &&
          *p.sub((*mb).nllen as usize) as u32 == (*mb).nl[0] as u32 &&
          ((*mb).nllen == 1 || *p.sub((*mb).nllen as usize - 1 + 1 - 1 + 1) as u32 == (*mb).nl[1] as u32)
      } )
```
For `WAS_NEWLINE`, the second byte test in C is
`*(p - NLBLOCK->nllen + 1) == NLBLOCK->nl[1]`, i.e.
`*p.sub((*mb).nllen as usize).add(1) as u32 == (*mb).nl[1] as u32`.

## Other helpers available

* `crate::matcher::{do_callout, match_ref, recurse_update_offsets, rep_min,
  rep_max, rep_typ, REPTYPE_MIN, REPTYPE_MAX, REPTYPE_POS, RECURSE_UNSET,
  GF_CAPTURE, GF_NOCAPTURE, GF_CONDASSERT, GF_RECURSE, GF_IDMASK, GF_DATAMASK,
  MATCH_MATCH, MATCH_NOMATCH, MATCH_ACCEPT, MATCH_KETRPOS, MATCH_COMMIT,
  MATCH_PRUNE, MATCH_SKIP, MATCH_SKIP_ARG, MATCH_THEN, MATCH_BACKTRACK_MIN,
  MATCH_BACKTRACK_MAX}` — all already `use`d by `matcher_core.rs`, so just use
  the bare names.
* `crate::extuni::_pcre2_extuni_8`, `crate::xclass::_pcre2_xclass_8`,
  `crate::xclass::_pcre2_eclass_8`, `crate::script_run::_pcre2_script_run_8`,
  `crate::newline::{_pcre2_is_newline_8, _pcre2_was_newline_8}`,
  `crate::string_utils::*`, `crate::tables::_pcre2_OP_lengths_8`,
  `crate::ucd_data::*`, `crate::ucp::*` (bare names available), the UCD helpers
  and `GET`/`GET2`/`getutf8`/`getutf8inc`/`utf8_extra` from `crate::internal`.
* `OP_*`, `PCRE2_*`, `PT_*`, `ESC_*`, `IMM2_SIZE`, `LINK_SIZE` — bare names.

## Rules

* LITERAL transliteration; byte-identical behaviour. Do not restructure logic,
  do not fix bugs, do not reorder tests.
* Omit `#ifdef SUPPORT_JIT`, `#ifdef PCRE2_DEBUG`, `#ifdef EBCDIC`,
  `#ifdef DEBUG_*`, `#if PCRE2_CODE_UNIT_WIDTH != 8`.
  Include `#ifdef SUPPORT_UNICODE`, `#ifdef SUPPORT_WIDE_CHARS`,
  `#ifdef MAYBE_UTF_MULTI`, `#if PCRE2_CODE_UNIT_WIDTH == 8`.
* `HSPACE_CASES` / `VSPACE_CASES` / `HSPACE_BYTE_CASES` /
  `HSPACE_MULTIBYTE_CASES` / `VSPACE_BYTE_CASES` / `VSPACE_MULTIBYTE_CASES`
  expand to (non-EBCDIC):
  - HSPACE_BYTE_CASES: `0x09 | 0x20 | 0xa0`
  - HSPACE_MULTIBYTE_CASES: `0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f | 0x205f | 0x3000`
  - HSPACE_CASES = both
  - VSPACE_BYTE_CASES: `0x0a | 0x0b | 0x0c | 0x0d | 0x85`
  - VSPACE_MULTIBYTE_CASES: `0x2028 | 0x2029`
  - VSPACE_CASES = both
