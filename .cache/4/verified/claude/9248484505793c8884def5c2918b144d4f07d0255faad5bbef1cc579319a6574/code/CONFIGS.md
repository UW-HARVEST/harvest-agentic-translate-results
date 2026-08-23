# CONFIGS.md — Configuration-surface table (VALID inputs)

**Provenance.** Derived mechanically from the C sources in `c_src/` for the build actually
compiled: `PCRE2_CODE_UNIT_WIDTH == 8`, `SUPPORT_UNICODE` defined (⇒ `SUPPORT_WIDE_CHARS`,
`MAYBE_UTF_MULTI`), **no** `SUPPORT_JIT`, no EBCDIC, no `PCRE2_DEBUG`, `LINK_SIZE == 2`,
`IMM2_SIZE == 2`. Files read: `c_src/include/pcre2.h` (option bits, INFO/CONFIG codes),
`pcre2_compile.c` / `pcre2_compile_class.c` / `pcre2_compile_cgroup.c` / `pcre2_compile.h`,
`pcre2_match.c`, `pcre2_dfa_match.c`, `pcre2_substitute.c`, `pcre2_substring.c`,
`pcre2_pattern_info.c`, `pcre2_serialize.c`, `pcre2_convert.c`, `pcre2_context.c`,
`pcre2_config.c`, `pcre2_match_data.c`, `pcre2_match_next.c`, `pcre2_study.c`,
`pcre2_auto_possess.c`, `pcre2_xclass.c`, `pcre2_script_run.c`, `pcre2_extuni.c`,
`pcre2_newline.c`, `pcre2_valid_utf.c`, `pcre2_ord2utf.c`, `pcre2_string_utils.c`,
`pcre2_find_bracket.c`, `pcre2_maketables.c`, `pcre2_chartables.c`, `pcre2_chkdint.c`,
`pcre2_tables.c`, `pcre2_error.c`, `pcre2_jit_misc_inc.h`, `pcre2_jit_match_inc.h`,
`pcre2_jit_compile.c` (no-JIT arms only), `pcre2_internal.h`, `pcre2_intmodedep.h`,
`config.h`. The 143-symbol export list is taken from `SYMBOLS.md`.

Build constants that rows below depend on: `MATCH_LIMIT = 10000000`,
`MATCH_LIMIT_DEPTH = MATCH_LIMIT`, `HEAP_LIMIT = 20000000` (KiB), `PARENS_NEST_LIMIT = 250`,
`MAX_VARLOOKBEHIND = 255`, `NEWLINE_DEFAULT = 2` (LF), `BSR_DEFAULT = UNICODE`,
`MAX_NAME_SIZE = 128`, `MAX_NAME_COUNT = 10000`, `MAX_MARK = 255`,
`MAX_PATTERN_SIZE = 1<<16`, `MAX_UTF_SINGLE_CU = 127`, `REQ_CU_MAX = 5000`,
`COMPILE_WORK_SIZE = 6000` code units, `PARSED_PATTERN_DEFAULT_SIZE = 1024`,
`GROUPINFO_DEFAULT_SIZE = 256`, `NAMED_GROUP_LIST_SIZE = 20`, `ECLASS_NEST_LIMIT = 15`,
`MAX_CACHE_BACKREF = 128`, `START_FRAMES_SIZE = 20480`, `DFA_START_RWS_SIZE = 30720` bytes
(= 7680 ints), `TABLES_LENGTH = ctypes_offset + 256`, `PCRE2_OPTIMIZATION_ALL = 0x7`.

This table is the mirror of an error table: every row is a **legal** call (or a legal call
sequence) that the C takes a *distinct* branch for. Rows are combinations, not single
options, because the interesting behaviour lives in the interaction.

## Axes

Derived from the `if`/`switch`/`#ifdef` sites in the C, not from guesses:

**A1 — compile option bits** (`PUBLIC_COMPILE_OPTIONS`, `pcre2_compile.c:693`): ANCHORED,
NO_UTF_CHECK, ENDANCHORED, ALLOW_EMPTY_CLASS, ALT_BSUX, AUTO_CALLOUT, CASELESS,
DOLLAR_ENDONLY, DOTALL, DUPNAMES, EXTENDED, EXTENDED_MORE, FIRSTLINE, MATCH_UNSET_BACKREF,
MULTILINE, NEVER_UCP, NEVER_UTF, NO_AUTO_CAPTURE, NO_AUTO_POSSESS, NO_DOTSTAR_ANCHOR,
NO_START_OPTIMIZE, UCP, UNGREEDY, UTF, NEVER_BACKSLASH_C, ALT_CIRCUMFLEX, ALT_VERBNAMES,
USE_OFFSET_LIMIT, LITERAL, MATCH_INVALID_UTF, ALT_EXTENDED_CLASS. Note the reduced
`PUBLIC_LITERAL_COMPILE_OPTIONS` subset when LITERAL is set.

**A2 — compile EXTRA option bits** (`PUBLIC_COMPILE_EXTRA_OPTIONS`, `:706`):
ALLOW_SURROGATE_ESCAPES, BAD_ESCAPE_IS_LITERAL, MATCH_WORD, MATCH_LINE, ESCAPED_CR_IS_LF,
ALT_BSUX, ALLOW_LOOKAROUND_BSK, CASELESS_RESTRICT, ASCII_BSD, ASCII_BSS, ASCII_BSW,
ASCII_POSIX, ASCII_DIGIT, PYTHON_OCTAL, NO_BS0, NEVER_CALLOUT, TURKISH_CASING.

**A3 — in-pattern option setters**: the `pso_list` verbs (`pcre2_compile.c:740-763`):
`(*UTF)` `(*UTF8)` `(*UCP)` `(*NOTEMPTY)` `(*NOTEMPTY_ATSTART)` `(*NO_AUTO_POSSESS)`
`(*NO_DOTSTAR_ANCHOR)` `(*NO_JIT)` `(*NO_START_OPT)` `(*CASELESS_RESTRICT)`
`(*TURKISH_CASING)` `(*LIMIT_HEAP=n)` `(*LIMIT_MATCH=n)` `(*LIMIT_DEPTH=n)`
`(*LIMIT_RECURSION=n)` `(*CR)` `(*LF)` `(*CRLF)` `(*ANY)` `(*ANYCRLF)` `(*NUL)`
`(*BSR_ANYCRLF)` `(*BSR_UNICODE)`; plus the inline `(?imnsxxUJr-…)`, `(?a)` `(?aD)` `(?aP)`
`(?aS)` `(?aT)` `(?aW)`, `(?^…)`, `(?|…)`. There is **no** `(*ALT_EXTENDED_CLASS)` verb.

**A4 — match option bits** (`PUBLIC_MATCH_OPTIONS = 0xE0044037`): ANCHORED, ENDANCHORED,
NOTBOL, NOTEOL, NOTEMPTY, NOTEMPTY_ATSTART, NO_UTF_CHECK, PARTIAL_SOFT, PARTIAL_HARD,
NO_JIT (accepted but never tested — pure no-op with no JIT), COPY_MATCHED_SUBJECT,
DISABLE_RECURSELOOP_CHECK. DFA's `PUBLIC_DFA_MATCH_OPTIONS = 0xE00040FF` swaps NO_JIT and
DISABLE_RECURSELOOP_CHECK for DFA_SHORTEST + DFA_RESTART.

**A5 — substitute option bits** (`SUBSTITUTE_OPTIONS`): GLOBAL, EXTENDED, UNSET_EMPTY,
UNKNOWN_UNSET, OVERFLOW_LENGTH, LITERAL, MATCHED, REPLACEMENT_ONLY (+ the A4 bits passed
through to `pcre2_match`).

**A6 — convert option bits**: exactly one of POSIX_BASIC (0x04) / POSIX_EXTENDED (0x08) /
GLOB (0x10) after masking with `TYPE_OPTIONS = 0x1c`, plus any subset of UTF (0x01),
NO_UTF_CHECK (0x02), GLOB_NO_WILD_SEPARATOR's extra bit (0x20), GLOB_NO_STARSTAR's extra
bit (0x40).

**A7 — context state**: newline convention (CR/LF/CRLF/ANY/ANYCRLF/NUL — 6 values),
`\R` convention (UNICODE/ANYCRLF), `max_varlookbehind`, `parens_nest_limit`,
`max_pattern_length`, `max_pattern_compiled_length`, `extra_options`, character tables
(`_pcre2_default_tables_8` vs `pcre2_maketables`), `optimization_flags` (NONE / FULL /
AUTO_POSSESS(±) / DOTSTAR_ANCHOR(±) / START_OPTIMIZE(±)), compile recursion guard,
match/depth/heap limits, offset limit, callout, substitute callout, substitute case
callout, glob separator (`/` `\` `.`), glob escape (0 or one of 32 punct chars), custom
malloc/free via general context.

**A8 — pattern shape**: length 0 / 1 / many / `PCRE2_ZERO_TERMINATED` / embedded NUL;
ASCII vs multi-byte UTF-8; capture count 0 / 1 / many / >128 (groupinfo heap) / >65535;
named-group count 0 / 1 / >20 (list heap) / duplicates; nesting depth vs
`parens_nest_limit`; parsed-pattern size vs 1024; opcode families (OP_CHAR/CHARI/NOT/NOTI,
OP_CLASS/NCLASS/XCLASS/ECLASS, backrefs, recursion, atomic, possessive, fixed and variable
lookbehind, `\X`, script runs, `(*scs:)`, callouts, verbs/`(*MARK)`, conditions).

**A9 — subject shape**: length 0 / 1 / many / `PCRE2_ZERO_TERMINATED`; `subject == NULL`
with length 0 (⇒ internal `null_str`); ASCII vs multi-byte UTF-8; invalid UTF-8;
newline data present (LF / CR / CRLF / VT / FF / NEL / LS / PS); startoffset
0 / mid-character-boundary / mid-character / `== length`.

**A10 — ovector / match_data shape**: `oveccount` 1 (the API floor) / < top_bracket+1 /
exactly top_bracket+1 / oversized / UINT16_MAX clamp; from-pattern vs explicit;
reuse across calls; heapframes grown vs reused; DFA "many ends, one start" semantics.

**A11 — limits and buffers**: match/depth/heap limit from context vs pattern vs both (min
wins); `offset_limit` with/without USE_OFFSET_LIMIT; DFA `wscount` (20 / `(wscount-2)/6`
capacity / restart bound `(wscount-2)/3`); substitute/convert/substring output buffer
0 / one-too-small / exactly-enough / oversized / NULL two-pass.

**A12 — low-level helper arguments**: `_pcre2_valid_utf_8` sequence lengths 1–6 and every
`UTF8_ERR` shape; `_pcre2_ord2utf_8` across the 6 `utf8_table1` bands;
`_pcre2_is_newline_8`/`_pcre2_was_newline_8` × {NLTYPE_ANY, NLTYPE_ANYCRLF} × {utf, !utf};
`_pcre2_extuni_8` grapheme-break table rows; `_pcre2_script_run_8` state machine
(UNSET/MAP/HANPENDING/HANHIRAKATA/HANBOPOMOFO/HANHANGUL + digit-set check);
`_pcre2_xclass_8` × {XCL_NOT, XCL_MAP, XCL_PROP, legacy list, packed char list} ×
code-point band; `_pcre2_eclass_8` RPN operators; `_pcre2_check_escape_8` escape classes ×
`isclass` × `cb == NULL`; `_pcre2_find_bracket_8` opcode-skip families;
`_pcre2_ckd_smul_8` overflow boundary; `_pcre2_memctl_malloc_8` with/without memctl.

**A13 — serialized-code round trips**: 1 code / many codes / shared vs mixed tables /
`number_of_codes` larger than the stream / default-vs-custom general context.

---

### 1. compile — `pcre2_compile_8` (+ `pcre2_code_copy_8`, `pcre2_code_copy_with_tables_8`, `pcre2_code_free_8`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `pcre2_compile_8` | options 0, xoptions 0, ccontext NULL, pattern `abc` len 3. Expect FIRSTSET `'a'`, LASTSET `'c'`, MINLENGTH 3, no FIRSTBITMAP, top_bracket 0. | [ ] |
| 2 | `pcre2_compile_8` | options 0, pattern `abc\0` with `patlen = PCRE2_ZERO_TERMINATED`; must be byte-identical to row 1. | [ ] |
| 3 | `pcre2_compile_8` | `pattern == NULL`, `patlen == 0` (⇒ internal `null_str` `{0xcd}`), options 0. Empty pattern: MATCHEMPTY set, MINLENGTH 0, top_bracket 0. | [ ] |
| 4 | `pcre2_compile_8` | non-NULL pointer, `patlen == 0`, options 0 — same compiled bytes as row 3. | [ ] |
| 5 | `pcre2_compile_8` | pattern `a\x00b` with explicit `patlen = 3`: NUL is a literal `OP_CHAR 0x00`. | [ ] |
| 6 | `pcre2_compile_8` | 40-char literal run `abcdefghij...`: 40 × `OP_CHAR`, FIRSTSET `'a'`, LASTSET last char, MINLENGTH 40. | [ ] |
| 7 | `pcre2_compile_8` | `PCRE2_ANCHORED`, pattern `abc`: `LASTSET` must be **off** (`:11218` requires `REQ_VARY` when anchored). | [ ] |
| 8 | `pcre2_compile_8` | `PCRE2_ANCHORED`, pattern `a.*c`: reqcu follows a variable item ⇒ `REQ_VARY` ⇒ `LASTSET` on with `'c'`. | [ ] |
| 9 | `pcre2_compile_8` | options 0, `^abc` (no MULTILINE): `is_anchored` auto-sets `PCRE2_ANCHORED` in ALLOPTIONS; FIRSTCODETYPE 0 (STARTLINE gate fails). | [ ] |
| 10 | `pcre2_compile_8` | options `PCRE2_DOTALL`, `.*abc`: `OP_ALLANY` ⇒ dotstar auto-anchor ⇒ ALLOPTIONS has ANCHORED. | [ ] |
| 11 | `pcre2_compile_8` | `PCRE2_DOTALL\|PCRE2_NO_DOTSTAR_ANCHOR`, `.*abc`: clears `PCRE2_OPTIM_DOTSTAR_ANCHOR` ⇒ **not** anchored. | [ ] |
| 12 | `pcre2_compile_8` | options 0, `.*abc` (`OP_ANY`): `is_startline` ⇒ `PCRE2_STARTLINE`, FIRSTCODETYPE 2, FIRSTBITMAP NULL. | [ ] |
| 13 | `pcre2_compile_8` | `PCRE2_MULTILINE`, `^a`: `OP_CIRCM`, firstcuflags forced `REQ_NONE` at `:6260` ⇒ FIRSTCODETYPE 2, not 1. | [ ] |
| 14 | `pcre2_compile_8` | `PCRE2_MULTILINE`, `a$`: `OP_DOLLM` (vs `OP_DOLL` without). | [ ] |
| 15 | `pcre2_compile_8` | `(?m)^a$` — in-pattern MULTILINE via `META_OPTIONS`, must equal row 13+14 codegen. | [ ] |
| 16 | `pcre2_compile_8` | `PCRE2_CASELESS`, `a`: `OP_CHARI`, `PCRE2_FIRSTCASELESS` because `cb->fcc['a'] != 'a'`. | [ ] |
| 17 | `pcre2_compile_8` | `PCRE2_CASELESS\|PCRE2_UTF`, `k`: multi-case set ⇒ `OP_PROP PT_CLIST <caseset>` (K/k/U+212A). | [ ] |
| 18 | `pcre2_compile_8` | `PCRE2_CASELESS\|PCRE2_UTF` + `PCRE2_EXTRA_CASELESS_RESTRICT`, `k`: caseset first member < 128 ⇒ plain `OP_CHARI`. | [ ] |
| 19 | `pcre2_compile_8` | `PCRE2_CASELESS\|PCRE2_UCP` **without** UTF, literal byte `0xFF`: UCP path via `UCD_OTHERCASE`, 8-bit only. | [ ] |
| 20 | `pcre2_compile_8` | `PCRE2_CASELESS\|PCRE2_UTF`, `[^k]`: one-char negated class ⇒ `OP_NOTPROP PT_CLIST`. | [ ] |
| 21 | `pcre2_compile_8` | options 0, `[Aa]`: two-char case-partner fold (`:6416-6455`) ⇒ `OP_CHARI` with `reset_caseful` undone at `:8523`. | [ ] |
| 22 | `pcre2_compile_8` | options 0, `[Ab]`: **not** a case pair ⇒ stays `OP_CLASS` with a 32-byte bitmap. | [ ] |
| 23 | `pcre2_compile_8` | `PCRE2_CASELESS\|PCRE2_UCP` + `EXTRA_CASELESS_RESTRICT`, `[Kk]`: fold happens (restrict suppresses the caseset). | [ ] |
| 24 | `pcre2_compile_8` | `PCRE2_UTF` + `PCRE2_EXTRA_TURKISH_CASING` + `(?i)`, pattern `i`: Turkish dotted-I caseset from `_pcre2_ucd_turkish_dotted_i_caseset_8`. | [ ] |
| 25 | `pcre2_compile_8` | `PCRE2_UTF\|PCRE2_CASELESS` + `EXTRA_TURKISH_CASING`, `[Ii]`: must **not** case-fold to `OP_CHARI`. | [ ] |
| 26 | `pcre2_compile_8` | `PCRE2_CASELESS`, `(a)\1`: `OP_REFI` + REFI flags byte 0. Repeat with `EXTRA_CASELESS_RESTRICT` (flag bit) and with `UTF+EXTRA_TURKISH_CASING` (flag bit) — 3 distinct flag bytes. | [ ] |
| 27 | `pcre2_compile_8` | `PCRE2_DUPNAMES`, `(?<a>x)(?<a>y)\k<a>`: `OP_DNREF`, NAMECOUNT 2, both slots present, NAMEENTRYSIZE = 1+2+1. | [ ] |
| 28 | `pcre2_compile_8` | `PCRE2_DUPNAMES\|PCRE2_CASELESS`, `(?<a>x)(?<a>y)\k<a>`: `OP_DNREFI` + flags byte. | [ ] |
| 29 | `pcre2_compile_8` | options 0, `(?J)(?<a>x)(?<a>y)`: JCHANGED set in flags (API DUPNAMES does **not** set it). | [ ] |
| 30 | `pcre2_compile_8` | options 0, `(?\|(?<a>x)\|(?<a>y))`: same group number ⇒ duplicate name accepted without DUPNAMES; `PCRE2_DUPCAPUSED` set. | [ ] |
| 31 | `pcre2_compile_8` | options 0, `(?<b>x)(?<a>y)`: out-of-order name-table insert ⇒ `memmove` path in `_pcre2_compile_add_name_to_table8`. | [ ] |
| 32 | `pcre2_compile_8` | options 0, `(?<ab>x)(?<a>y)`: substring-name case (`crc == 0 && slot[IMM2_SIZE+length] != 0` ⇒ `crc = -1`). | [ ] |
| 33 | `pcre2_compile_8` | 21 distinct named groups `(?<n01>a)...(?<n21>a)`: forces the `named_groups` heap realloc at `:5769` and free at `:11289`. | [ ] |
| 34 | `pcre2_compile_8` | name lengths 1 and 128 (`MAX_NAME_SIZE`): `(?<a>x)` and a 128-char name; NAMEENTRYSIZE = 131. | [ ] |
| 35 | `pcre2_compile_8` | UTF group name: `PCRE2_UTF`, `(?<\xC3\xA9>a)\k<\xC3\xA9>` (ucp_L path in `read_name`). | [ ] |
| 36 | `pcre2_compile_8` | `PCRE2_EXTENDED`, `a b\t# comment\nc` with `pcre2_set_newline(LF)`: comment ends at `\n`, whitespace dropped. | [ ] |
| 37 | `pcre2_compile_8` | `PCRE2_EXTENDED` + `pcre2_set_newline(CR)`, `a# c\rb`; and + `(CRLF)` with `\r\n`; and + `(ANY)` with `\x0b`; and + `(NUL)` with `\x00` — 4 distinct `IS_NEWLINE`/`cb->nllen` outcomes for `#`-comment termination. | [ ] |
| 38 | `pcre2_compile_8` | `PCRE2_EXTENDED\|PCRE2_UTF`, pattern containing U+00A0/U+200E/U+2029 as `/x` whitespace (`:3453-3475` Unicode arm). | [ ] |
| 39 | `pcre2_compile_8` | `PCRE2_EXTENDED_MORE` (implies EXTENDED at `:3222`), `[a b]` and `[ ^a]`: space/HT skipped inside the class. | [ ] |
| 40 | `pcre2_compile_8` | `(?xx)[a b]` then `(?x:[a b])`: verify `(?x)` clears EXTENDED_MORE via `:5137-5139`. | [ ] |
| 41 | `pcre2_compile_8` | `PCRE2_UNGREEDY`, patterns `a*`, `a*?`, `a*+`, `a{2,4}`, `a{2,4}?`, `a{2,4}+`: greedy/minimal swap via `greedy_default`/`greedy_non_default`. | [ ] |
| 42 | `pcre2_compile_8` | `(?U)a*` then `(?U)(?-U)a*`: inline UNGREEDY set and unset. | [ ] |
| 43 | `pcre2_compile_8` | `PCRE2_NO_AUTO_CAPTURE`, `(a)(?<n>b)`: plain `(` ⇒ `META_NOCAPTURE`, named group still captures ⇒ top_bracket 1. | [ ] |
| 44 | `pcre2_compile_8` | `(?n)(?<n>a)\k<n>`: inline NO_AUTO_CAPTURE + named backref. | [ ] |
| 45 | `pcre2_compile_8` | options 0 vs `PCRE2_NO_AUTO_POSSESS`, patterns `a+b`, `\d+\D`, `[a-z]+[0-9]`, `\w+\s`, `x+\z`: possessified vs left alone. | [ ] |
| 46 | `pcre2_compile_8` | `(*NO_AUTO_POSSESS)a+b`: verb sets both `optim_flags &= ~AUTO_POSSESS` and `PCRE2_NO_AUTO_POSSESS` in ALLOPTIONS. | [ ] |
| 47 | `pcre2_compile_8`, `pcre2_set_optimize_8` | ccontext with `PCRE2_OPTIMIZATION_NONE`, probe set {`a+b`, `(?s).*x`, `abc`, `[Ww]ord`}: no possessify, no dotstar anchor, no firstcu/bitmap/minlength. | [ ] |
| 48 | `pcre2_compile_8`, `pcre2_set_optimize_8` | `PCRE2_OPTIMIZATION_NONE` then `PCRE2_AUTO_POSSESS` (64) — bit 0 back on only; same probe set. | [ ] |
| 49 | `pcre2_compile_8`, `pcre2_set_optimize_8` | `PCRE2_AUTO_POSSESS_OFF` (65) / `PCRE2_DOTSTAR_ANCHOR_OFF` (67) / `PCRE2_START_OPTIMIZE_OFF` (69) each from the FULL default; and `PCRE2_OPTIMIZATION_FULL` restore. | [ ] |
| 50 | `pcre2_compile_8` | `PCRE2_NO_START_OPTIMIZE`, `abc`: FIRSTCODETYPE 0, LASTCODETYPE 0, FIRSTBITMAP NULL, MINLENGTH 0 — but auto-ANCHORED for `^abc` still applies. | [ ] |
| 51 | `pcre2_compile_8` | `(*NO_START_OPT)abc` — verb form of row 50. | [ ] |
| 52 | `pcre2_compile_8` | `PCRE2_ALLOW_EMPTY_CLASS`, patterns `[]` (⇒ `OP_CLASS` + 32 zero bytes), `[^]` (⇒ `OP_ALLANY`), `[]a]`, `[]]*`. | [ ] |
| 53 | `pcre2_compile_8` | options 0, `[]a]`: `]` is a literal class member (`RANGE_OK_LITERAL`) — contrast with row 52. | [ ] |
| 54 | `pcre2_compile_8` | `PCRE2_ALT_BSUX`, patterns `A`, `\x41`, `\U`: 4-hex `\u`, 2-hex `\x`, literal `U`. | [ ] |
| 55 | `pcre2_compile_8` | `PCRE2_EXTRA_ALT_BSUX` only, patterns `\u{41}`, `\u{ 12}`, `\u{}` (⇒ `ESC_ub` ⇒ literal `u` + `{}`), and `[\u{}]` (in-class ⇒ literal `u`). | [ ] |
| 56 | `pcre2_compile_8` | `PCRE2_UTF\|PCRE2_ALT_BSUX` + `PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES`, `\ud800`; also `\o{155000}` and `\x{d800}` with the same extra bit. | [ ] |
| 57 | `pcre2_compile_8` | `PCRE2_AUTO_CALLOUT`, `abc`: callout number 255 before each item + trailing callout; `parsed_size_needed` = 4×(len)+4. | [ ] |
| 58 | `pcre2_compile_8` | `PCRE2_AUTO_CALLOUT`, `a(?C1)b`: explicit callout abolishes the immediately preceding auto callout (`:5313`). | [ ] |
| 59 | `pcre2_compile_8` | `PCRE2_AUTO_CALLOUT\|PCRE2_LITERAL`, `a.b`: LITERAL fast path with `manage_callouts` at `:3196`. | [ ] |
| 60 | `pcre2_compile_8` | `PCRE2_AUTO_CALLOUT`, `(?(?=a)b\|c)`: `expect_cond_assert` interaction, callout between `OP_COND` and the assertion. | [ ] |
| 61 | `pcre2_compile_8` | `PCRE2_EXTRA_NEVER_CALLOUT` + `PCRE2_AUTO_CALLOUT`, pattern with no `(?C`: auto callouts still emitted. | [ ] |
| 62 | `pcre2_compile_8` | callout string delimiters, one pattern each: `(?C` + `` ` `` / `'` / `"` / `^` / `%` / `#` / `$` / `{` (closing `}`); plus a doubled delimiter `(?C"a""b")`. | [ ] |
| 63 | `pcre2_compile_8` | numeric callouts `(?C)`, `(?C0)`, `(?C1)`, `(?C255)`: `OP_CALLOUT` with number 0/0/1/255. | [ ] |
| 64 | `pcre2_compile_8` | `PCRE2_LITERAL`, pattern `a.b*c[` — every metachar literal, no class parsing, no `nest_depth`. | [ ] |
| 65 | `pcre2_compile_8` | `PCRE2_LITERAL\|PCRE2_CASELESS` + `PCRE2_EXTRA_MATCH_WORD`: `\b(?:a.b)\b` wrapping around literal text. | [ ] |
| 66 | `pcre2_compile_8` | `PCRE2_LITERAL\|PCRE2_MULTILINE` + `PCRE2_EXTRA_MATCH_LINE`: `^(?:…)$` with `OP_CIRCM`/`OP_DOLLM`. | [ ] |
| 67 | `pcre2_compile_8` | `PCRE2_LITERAL`, pattern `(*UTF)x`: `pso_list` scan skipped ⇒ verb text is literal. | [ ] |
| 68 | `pcre2_compile_8` | `PCRE2_LITERAL`, pattern `a\x00b` len 3 with `PCRE2_EXTRA_CASELESS_RESTRICT` (a legal LITERAL extra bit). | [ ] |
| 69 | `pcre2_compile_8` | `PCRE2_EXTRA_ESCAPED_CR_IS_LF`, `a\rb` (escaped CR ⇒ LF, HASCRORLF set) vs `a\x0db` (unaffected, still CR). | [ ] |
| 70 | `pcre2_compile_8` | `PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL`, patterns `\q`, `\y`, `\F`, `\L`, `[\q]`, `\x{`; and `\C` with `PCRE2_NEVER_BACKSLASH_C` (ERR83 rescued to literal `C`); and `\0` with `PCRE2_EXTRA_NO_BS0`. | [ ] |
| 71 | `pcre2_compile_8` | `PCRE2_EXTRA_PYTHON_OCTAL` with `(a)(b)`-prefixed patterns `\12`, `\123`, `\1`, `\8`, `\377`: 3-octal-digit rule vs backref. | [ ] |
| 72 | `pcre2_compile_8` | default (Perl) octal/backref disambiguation, same patterns as row 71: `s < 10 \|\| c >= '8' \|\| s <= bracount`. | [ ] |
| 73 | `pcre2_compile_8` | `PCRE2_EXTRA_NO_BS0`, patterns `\00`, `\000`, `\x00`, `\o{0}` — all still legal (only bare `\0` is rejected). | [ ] |
| 74 | `pcre2_compile_8` | `PCRE2_UCP` alone, patterns `\d \D \s \S \w \W \b \B`: `META_ESCAPE+ESC_p` with `PT_PC/ucp_Nd`, `PT_SPACE`, `PT_WORD`, and `OP_UCP_WORD_BOUNDARY`. | [ ] |
| 75 | `pcre2_compile_8` | `PCRE2_UCP\|PCRE2_EXTRA_ASCII_BSD`, `\d\D` stay `OP_DIGIT`/`OP_NOT_DIGIT`; repeat inside a class `[\d]`. | [ ] |
| 76 | `pcre2_compile_8` | `PCRE2_UCP\|PCRE2_EXTRA_ASCII_BSS`, `\s\S` stay ASCII bitmaps. | [ ] |
| 77 | `pcre2_compile_8` | `PCRE2_UCP\|PCRE2_EXTRA_ASCII_BSW`, `\w\W` **and** `\b\B` stay ASCII (`:8372` covers both). | [ ] |
| 78 | `pcre2_compile_8` | `PCRE2_UCP\|PCRE2_EXTRA_ASCII_POSIX`, all 14 `[[:name:]]` classes revert to `cb->cbits`. | [ ] |
| 79 | `pcre2_compile_8` | `PCRE2_UCP\|PCRE2_EXTRA_ASCII_DIGIT`, only `[[:digit:]]` and `[[:xdigit:]]` revert; `[[:alpha:]]` still `\p{L}`. | [ ] |
| 80 | `pcre2_compile_8` | `PCRE2_UCP` with all 14 POSIX classes ⇒ the `posix_substitutes[]` table: alpha/lower/upper/alnum/ascii/blank/cntrl/digit/graph/print/punct/space/word/xdigit (incl. `[[:blank:]]` ⇒ `\h`, `[[:^blank:]]` ⇒ `\H`, `[[:ascii:]]` falling through). | [ ] |
| 81 | `pcre2_compile_8` | `PCRE2_UCP`, `[[:<:]]a` and `[[:>:]]a` (word-boundary POSIX aliases ⇒ `ESC_p`/`PT_WORD`) vs options 0 (⇒ `ESC_w`). | [ ] |
| 82 | `pcre2_compile_8` | `PCRE2_UTF`, one pattern per `PT_*`: `\p{L&}`(LAMP), `\p{L}`(GC), `\p{Lu}`(PC), `\p{Han}`(SC), `\p{scx:Han}`(SCX), `\p{Xan}`(ALNUM), `\p{Xps}`(SPACE), `\p{Xsp}`(PXSPACE), `\p{Xwd}`(WORD), `\p{Xuc}`(UCNC), `\p{bc=AL}`(BIDICL), `\p{Bidi_Control}`(BOOL), `\p{Any}`(ANY); plus `\P{L}`, `\p{^L}`, `\pL`, `\p{ l _ }` loose-match. | [ ] |
| 83 | `pcre2_compile_8` | `PCRE2_UTF\|PCRE2_CASELESS`, `\p{Lu}` ⇒ rewritten to `PT_LAMP`/pdata 0 (`:8304`); also `[\p{Ll}]` in-class at `:4547`. | [ ] |
| 84 | `pcre2_compile_8` | class opcode selection: `[abc]`/`[a-c]`/`[[:alpha:]]` ⇒ `OP_CLASS`; `[^abc]`/`[\D]`/`[\S]`/`[\W]` ⇒ `OP_NCLASS`; `[\s\S]` non-UTF and `[\x00-\xff]` non-UTF ⇒ `OP_ALLANY`. | [ ] |
| 85 | `pcre2_compile_8` | `PCRE2_UTF`, XCLASS shapes: `[\x{100}]` (plain), `[a\x{100}]` (`XCL_MAP`), `[\x{100}-\x{200}]` (`XCL_RANGE`), `[\p{L}]` (`XCL_HASPROP`), `[^\x{100}]` (`XCL_NOT` no map), `[^a\x{100}]` (`XCL_NOT`+`XCL_MAP`). | [ ] |
| 86 | `pcre2_compile_8` | `PCRE2_UTF`, `[\x{100}\x{102}\x{104}\x{106}\x{108}\x{10a}]` — ≥6 high ranges ⇒ `XCLASS_HAS_CHAR_LISTS` ⇒ `XCL_LIST` with the backwards char-list block and `char_lists_size` accounting. | [ ] |
| 87 | `pcre2_compile_8` | `PCRE2_UTF`, `[\W]` and `[^\x00-\xff]`: `XCLASS_HIGH_ANY` (last range ends at `MAX_UTF_CODE_POINT`) ⇒ `should_flip_negation`. | [ ] |
| 88 | `pcre2_compile_8` | class ranges crossing the byte boundaries: `[\x7f-\x81]`, `[\xfe-\x{101}]`+UTF, `[a-\xff]`, `[\x00-\xff]` (long-range `memset` path vs `[a-c]` short path). | [ ] |
| 89 | `pcre2_compile_8` | `PCRE2_CASELESS\|PCRE2_UCP`, `[a-z]` and `[[:upper:]]` (⇒ `[[:alpha:]]` via `posix_class <= 2`); and `PCRE2_CASELESS\|PCRE2_UTF`, `[\x{212a}]` (`utf_caseless_extend`). | [ ] |
| 90 | `pcre2_compile_8` | `PCRE2_ALT_EXTENDED_CLASS`, patterns `[a&&b]`, `[[a-z]&&[^aeiou]]`, `[a--b]`, `[\d\|\|\s]`, `[a~~b]`, `[[a][b]]`, `[a-z_ -- m]` (juxtaposition ⇒ implicit union). | [ ] |
| 91 | `pcre2_compile_8` | `PCRE2_ALT_EXTENDED_CLASS`, 14 levels of nested `[` (one below `ECLASS_NEST_LIMIT`). | [ ] |
| 92 | `pcre2_compile_8` | Perl extended class `(?[ [\p{L}] - [a-z] ])` and `(?[ \p{L} & \p{Latin} ])`: `CLASS_MODE_PERL_EXT`, `OP_ECLASS` with `ECL_MAP` + RPN body. | [ ] |
| 93 | `pcre2_compile_8` | eclass constant-folding back to a simpler opcode: `[a&&a]` ⇒ `OP_CLASS`, `[\W\|\|\w]` ⇒ `OP_ALLANY`, `(?[ [\p{L}] & [\x{100}-\x{200}] ])`+UTF ⇒ `OP_XCLASS` (`fold_binary`/`fold_negation` arms). | [ ] |
| 94 | `pcre2_compile_8` | quantifier grid on `a`: `a?` `a*` `a+` `a{0}` `a{1}` `a{3}` `a{2,}` `a{2,4}` `a{0,3}` — each also with `?` and `+` suffix (27 forms) ⇒ `OP_*QUERY/STAR/PLUS/UPTO/EXACT` × greedy/minimal/possessive. | [ ] |
| 95 | `pcre2_compile_8` | quantifier on each *previous-item* family: `[ab]*`, `\d{2,4}`, `.{2}`, `\p{L}{2,4}` (prop_type path), `\1{2,3}`, `(?:ab)+`, `(a){2,3}`, `(?R)+`, `(?=a)*`, `[ab]{0}` (the `code < last_code` rewind), `(a){0}`. | [ ] |
| 96 | `pcre2_compile_8` | possessive wrap paths: `a++` (⇒ `OP_POSPLUS` from `opcode_possessify`) vs `(?:ab)++`, `(a){2,3}+`, `\1++`, `[a-z]{2,}+`, `\p{L}++` (⇒ `OP_ONCE` wrapper). | [ ] |
| 97 | `pcre2_compile_8` | `a{ 2 , 4 }` — `read_repeat_counts` whitespace tolerance; and `x{65535}` (`MAX_REPEAT_COUNT`). | [ ] |
| 98 | `pcre2_compile_8` | group kinds: `(a)`⇒`OP_CBRA`, `(?:a)`⇒`OP_BRA`, `(?>a)`⇒`OP_ONCE`, `(?\|(a)\|(b))`⇒branch reset with `PCRE2_DUPCAPUSED`, `(?<n>a)`/`(?'n'a)`/`(?P<n>a)` all equivalent. | [ ] |
| 99 | `pcre2_compile_8` | backreference syntax matrix on `(a)(b)`: `\1`, `\2`, `\g1`, `\g{1}`, `\g{-1}`, `\g{ 1 }`, `\k<n>`, `\k'n'`, `\k{n}`, `\g{n}`, `(?P=n)` — with a named group added for the name forms. | [ ] |
| 100 | `pcre2_compile_8` | conditions: `(?(1)a\|b)`, `(?(+1)…)`, `(?(-1)…)`, `(?(<n>)…)`, `(?(n)…)`, `(?(R)…)`, `(?(R1)…)`, `(?(R&n)…)`, `(?(DEFINE)(a))`, `(?(VERSION>=10.0)a\|b)`, `(?(VERSION=10.0)…)`, `(?(?=a)b\|c)`; plus one-branch vs two-branch (`condcount` 1 forces `subfirstcuflags = REQ_NONE`). | [ ] |
| 101 | `pcre2_compile_8` | `(?<R>a)(?(R)b)` — named group shadowing the `R` recursion condition (name wins). | [ ] |
| 102 | `pcre2_compile_8` | `PCRE2_DUPNAMES`, `(?<a>x)(?<a>y)(?(<a>)z)` ⇒ `OP_DNCREF`; and `(?(R&a)…)` ⇒ `OP_DNRREF`. | [ ] |
| 103 | `pcre2_compile_8` | lookarounds: `(?=a)`, `(?!a)`, `(?!)` (⇒ `OP_FAIL` optimization), `(?!)*` (optimization suppressed), `(?<=ab)` (⇒ `OP_REVERSE 2`), `(?<=a\|bb)` (⇒ `OP_VREVERSE 1 2`), `(?<!a)`, `(?<=(?<=a)b)` nested. | [ ] |
| 104 | `pcre2_compile_8` | all 19 alpha-assertion names: `(*pla:)` `(*plb:)` `(*napla:)` `(*naplb:)` `(*nla:)` `(*nlb:)` `(*positive_lookahead:)` `(*positive_lookbehind:)` `(*non_atomic_positive_lookahead:)` `(*non_atomic_positive_lookbehind:)` `(*negative_lookahead:)` `(*negative_lookbehind:)` `(*scs:)` `(*scan_substring:)` `(*atomic:)` `(*sr:)` `(*asr:)` `(*script_run:)` `(*atomic_script_run:)`. | [ ] |
| 105 | `pcre2_compile_8`, `pcre2_set_max_varlookbehind_8` | `(?<=a{1,5})` with `max_varlookbehind` 5 (accepted) — and 255 default with `(?<=a{1,255})`; fixed-length `(?<=ab)` is exempt from the limit even at limit 0. | [ ] |
| 106 | `pcre2_compile_8` | `PCRE2_MATCH_UNSET_BACKREF`, `(a)(?<=\1)`: backref in a lookbehind becomes variable ⇒ `OP_VREVERSE`; contrast options 0 (fixed). | [ ] |
| 107 | `pcre2_compile_8` | `(?<=\R)` (min 1 / max 2), `(?<=\d)`, `(?<=(a\|bb))`, `(?<=(?1))(a)` (subroutine length), `(?<=\1)(a)` — `get_branchlength` item-length arms. | [ ] |
| 108 | `pcre2_compile_8` | `PCRE2_UTF`, `(?<=a)\C` — `\C` legal outside a lookbehind in UTF; and non-UTF `(?<=\C)` ⇒ `OP_ALLANY` so it is legal. | [ ] |
| 109 | `pcre2_compile_8` | recursion forms: `(?R)`, `(?R0)`, `(?0)`, `(?1)`, `(?+1)`, `(?-1)`, `(?&n)`, `(?P>n)`, `\g<1>`, `\g'1'`, `\g<n>` — with a matching group set. | [ ] |
| 110 | `pcre2_compile_8` | recursion with capture-argument lists: `(a)(b)(c)(?1(2,3))` and `(?<x>a)(?<y>b)(?&x(<y>))` ⇒ `OP_RECURSE` + `OP_CREF` chain via `_pcre2_compile_parse_recurse_args8` (heapsort + dedup). | [ ] |
| 111 | `pcre2_compile_8` | pattern with 10 distinct recursion targets plus repeats ⇒ exercises the 8-entry `recurse_cache` in `find_recurse` and the `cb->had_recurse` fixup pass. | [ ] |
| 112 | `pcre2_compile_8` | `PCRE2_UTF\|PCRE2_UCP`, `(*script_run:\p{L}+)` and `(*asr:\d+)` ⇒ `OP_SCRIPT_RUN`, `OP_ONCE`+`OP_SCRIPT_RUN` (`NSF_ATOMICSR`, two `META_KET`s). | [ ] |
| 113 | `pcre2_compile_8` | all verbs: `(*MARK:x)`, `(*:x)`, `(*ACCEPT)`, `(*ACCEPT:x)`, `(*F)`, `(*FAIL)`, `(*FAIL:x)`, `(*COMMIT)`, `(*COMMIT:x)`, `(*PRUNE)`, `(*PRUNE:x)`, `(*SKIP)`, `(*SKIP:x)`, `(*THEN)`, `(*THEN:x)`; plus a 255-code-unit MARK argument (`MAX_MARK`). | [ ] |
| 114 | `pcre2_compile_8` | `(a)(*ACCEPT)` and `(?=(*ACCEPT))`: `OP_CLOSE` insertion for open captures, `OP_ASSERT_ACCEPT` inside an assertion, `PCRE2_HASACCEPT` ⇒ reqcu cleared and MINLENGTH forced 0. | [ ] |
| 115 | `pcre2_compile_8` | `(*PRUNE)a.*b` and `(*SKIP)a.*b`: `cb->had_pruneorskip` disables both the `.*` anchor and the STARTLINE optimization. | [ ] |
| 116 | `pcre2_compile_8` | `(*THEN)` anywhere ⇒ `PCRE2_HASTHEN` flag (changes `mb->hasthen` and the `OP_BRA` fast path at match time). | [ ] |
| 117 | `pcre2_compile_8` | `PCRE2_ALT_VERBNAMES`, `(*MARK:a\x41b)`, `(*MARK:a\Qb)c\Ed)`, `(*MARK:\n)`; and `(?x)(*MARK:a b)` with ALT_VERBNAMES (whitespace stripped only when *both* EXTENDED and ALT_VERBNAMES are set). | [ ] |
| 118 | `pcre2_compile_8` | `(*scs:1)a` / `(*scan_substring:<n>)a` with the referenced group present ⇒ `OP_ASSERT_SCS` + `OP_CREF`/`OP_DNCREF`; `assert_depth` incremented. | [ ] |
| 119 | `pcre2_compile_8` | `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK`, `(?=\K)`, `(?<=a\K)`, `(*napla:\K)`, `(*scs:1)\K` ⇒ `PCRE2_HASBSK` flag set, no ERR99. | [ ] |
| 120 | `pcre2_compile_8` | escape coverage, one pattern each: `\X` `\R` `\N` `\Q…\E` (also lone `\E`, `\Q\E`) `\K` `\b` `\B` `\A` `\Z` `\z` `\G` `\h` `\H` `\v` `\V` `\a` `\e` `\f` `\n` `\r` `\t` `\$` `\\` `\.` `\[`. | [ ] |
| 121 | `pcre2_compile_8` | numeric escapes: `\o{101}`, `\o{ 101 }`, `\x{41}`, `\x{ 41 }`, `\x41`, `\x4`, `\101`, `\00`, `\000`, `\08`, `\cA`, `\c[`, `\c?` (⇒ 0x7f); `PCRE2_UTF` variants `\x{10FFFF}`, `\o{4177777}`, `\N{U+41}`. | [ ] |
| 122 | `pcre2_compile_8`, `pcre2_set_newline_8` | one compile per newline convention CR/LF/CRLF/ANY/ANYCRLF/NUL with pattern `a\r\nb$` under MULTILINE — verifies `re->newline_convention` storage and `PCRE2_INFO_NEWLINE`. | [ ] |
| 123 | `pcre2_compile_8`, `pcre2_set_bsr_8` | `\R` compiled with `PCRE2_BSR_UNICODE` and `PCRE2_BSR_ANYCRLF` (identical opcodes, different `re->bsr_convention` / `PCRE2_INFO_BSR`). | [ ] |
| 124 | `pcre2_compile_8` | every `pso_list` verb once: `(*UTF)`, `(*UTF8)`, `(*UCP)`, `(*NOTEMPTY)`, `(*NOTEMPTY_ATSTART)`, `(*NO_JIT)`, `(*CASELESS_RESTRICT)`, `(*TURKISH_CASING)` (with `(*UTF)`), `(*CR)`, `(*LF)`, `(*CRLF)`, `(*ANY)`, `(*ANYCRLF)`, `(*NUL)`, `(*BSR_ANYCRLF)`, `(*BSR_UNICODE)` — plus one stacked case `(*UTF)(*UCP)(*CR)abc`. | [ ] |
| 125 | `pcre2_compile_8` | `(*LIMIT_MATCH=100)(*LIMIT_DEPTH=50)(*LIMIT_HEAP=1000)x` and `(*LIMIT_RECURSION=50)x`: `PCRE2_INFO_MATCHLIMIT`/`DEPTHLIMIT`/`HEAPLIMIT` set; absent ⇒ `PCRE2_ERROR_UNSET` from `pcre2_pattern_info`. | [ ] |
| 126 | `pcre2_compile_8` | pattern starting with `(*ACCEPT)` — a `(*` that matches no `pso_list` entry, so the scan breaks and normal parsing takes over. | [ ] |
| 127 | `pcre2_compile_8`, `pcre2_set_max_pattern_length_8` | limit == patlen (accepted) and limit `PCRE2_UNSET` default; check the limit is applied *after* `PCRE2_ZERO_TERMINATED` resolution. | [ ] |
| 128 | `pcre2_compile_8`, `pcre2_set_max_pattern_compiled_length_8` | limit set to the exact `PCRE2_INFO_SIZE`-derived `re_blocksize` for a pattern with named groups + UTF char lists (note the limit excludes `sizeof(pcre2_real_code)`). | [ ] |
| 129 | `pcre2_compile_8`, `pcre2_set_parens_nest_limit_8` | 250 nested `(` at the default limit; limit 0 with pattern `a`; limit 1 with `(a)`. | [ ] |
| 130 | `pcre2_compile_8`, `pcre2_maketables_8`, `pcre2_set_character_tables_8` | compile `[[:alpha:]]`, `\w`, `[a-z]`+CASELESS, `(?i)a`, `[[:lower:]]` with `pcre2_maketables()` output instead of `_pcre2_default_tables_8`; then `pcre2_maketables_free_8` — verify `re->tables` pointer and `cb->fcc`-driven `PCRE2_FIRSTCASELESS`. | [ ] |
| 131 | `pcre2_compile_8`, `pcre2_set_compile_recursion_guard_8` | guard returning 0 always, pattern `((((a))))`: guard invoked once per group per pass (2 passes); guard receiving `cb->parens_depth`. | [ ] |
| 132 | `pcre2_compile_8` | 1100-char literal pattern ⇒ `parsed_pattern` heap allocation (`> PARSED_PATTERN_DEFAULT_SIZE`); also a 260-char pattern with `PCRE2_AUTO_CALLOUT` (×4 + 4) and a 1024/1025-boundary pair. | [ ] |
| 133 | `pcre2_compile_8` | 130 capture groups **plus** a lookbehind ⇒ `groupinfo` heap allocation (`bracount >= 128 && has_lookbehind`). | [ ] |
| 134 | `pcre2_compile_8` | pattern deep/long enough to exercise the compile-workspace safety margin (`WORK_SIZE_SAFETY_MARGIN`) without crossing it — ~240 nested groups with long bodies. | [ ] |
| 135 | `pcre2_compile_8` | `PCRE2_NEVER_UTF` with a non-UTF pattern, and `PCRE2_NEVER_UCP` with no UCP and no `(*UCP)` — both legal no-ops. | [ ] |
| 136 | `pcre2_compile_8` | `PCRE2_MATCH_INVALID_UTF` alone: implies `PCRE2_UTF` at `:10373`; ALLOPTIONS shows both. | [ ] |
| 137 | `pcre2_compile_8` | `PCRE2_UTF\|PCRE2_NO_UTF_CHECK` with a valid multi-byte pattern: `_pcre2_valid_utf_8` scan skipped, identical output to without. | [ ] |
| 138 | `pcre2_compile_8` | store-only options: `PCRE2_ENDANCHORED`, `PCRE2_DOLLAR_ENDONLY`, `PCRE2_ALT_CIRCUMFLEX`, `PCRE2_USE_OFFSET_LIMIT`, `PCRE2_FIRSTLINE` — verify they appear in ARGOPTIONS/ALLOPTIONS and change no compiled byte. | [ ] |
| 139 | `pcre2_compile_8` | inline option grid: `(?i)a`, `(?i)(?-i)a`, `(?i:a)b`, `(?^i)a`, `(?^)a`, `(?i-s:…)`, `(?x-i)`, `(?J)`, `(?r)`, `(?aD)`, `(?aP)`, `(?aS)`, `(?aT)`, `(?aW)`, `(?a)`, `(?-a)`, `(?)` (no `META_OPTIONS` emitted), `(?:)`. | [ ] |
| 140 | `pcre2_compile_8` | option scoping across groups: `(?i:(?-i:a)b)c` — `nest_save` push/restore of `PARSE_TRACKED_OPTIONS` and `PARSE_TRACKED_EXTRA_OPTIONS`. | [ ] |
| 141 | `pcre2_compile_8`, `_pcre2_study_8` | `find_firstassertedcu` adoption: `(?=abcde).+` (adopts `'a'`), `(?=a)b?a` (must **not** adopt, `assertedcu == reqcu`), `(?=abc)(?=abd)` (branches agree), `(?=abc)(?=xbc)` (disagree ⇒ 0). | [ ] |
| 142 | `pcre2_compile_8`, `_pcre2_study_8` | FIRSTBITMAP produced (`SSB_DONE`, >2 bits): `[abc]x`, `\d`, `a\|b\|c`, `\w+`, `[^\n]`. Assert the 32-byte bitmap contents. | [ ] |
| 143 | `pcre2_compile_8`, `_pcre2_study_8` | bitmap collapsed to FIRSTSET+FIRSTCASELESS (exactly 2 case-partner bits): `[Ww]ord`, `(word\|WORD)`, `[aA]x` — and the `PCRE2_LASTSET` clearing side effect for `[Aa]a` / `a*a`. | [ ] |
| 144 | `pcre2_compile_8`, `_pcre2_study_8` | bitmap abandoned (`SSB_FAIL`/`SSB_CONTINUE`): `.a`, `\Ca`+UTF, `\X`, `(?1)x` + `(x)`, `(a)\1`, `(?(1)a\|b)`, `(*MARK:x)a`, `[^\x{100}]`+UTF (XCL_NOT, no map), `[\p{L}\x{100}]` (XCL_HASPROP), `(?[a&&b])` (`OP_ECLASS`), `a?`, `a*b\|`. | [ ] |
| 145 | `pcre2_compile_8`, `_pcre2_study_8` | UTF-specific bitmap paths: `PCRE2_UTF` with `[\x{100}-\x{2000}]` (lead-byte bits), `\h`+UTF (bits C2/E1/E2/E3), `\v`/`\R`+UTF (bits C2/E2), `\D`+UTF (`set_nottype_bits` ⇒ bytes 0xC0-0xFF), `[^a]`+UTF (`OP_NCLASS` ⇒ `bitmap[24] \|= 0xf0` + `memset(+25,0xff,7)`); plus the `c > 127` collapse veto. | [ ] |
| 146 | `pcre2_compile_8`, `_pcre2_study_8` | MINLENGTH drivers: `abc`(3), `ab\|cdef`(2), `a*`(0, MATCHEMPTY), `(*ACCEPT)a`(0, HASACCEPT), `(abc)\1{3}`(12), `a{2,4}b`(3), `(a\|b(?1))` (recursion ignored), `(?(DEFINE)(a))b`, `(a){0}b`, `\C`+UTF (⇒ minminlength only), 129 backrefs (`top_backref > 128` ⇒ 0), `PCRE2_MATCH_UNSET_BACKREF` with `(a)\1`. | [ ] |
| 147 | `pcre2_compile_8` | HASCRORLF: `a\rb`, `a\nb`, `[\r]`, `[\x00-\x0d]`, `\x0d` (all set) vs `\R` (not set); `\r`+`EXTRA_ESCAPED_CR_IS_LF` (set, via LF). | [ ] |
| 148 | `pcre2_compile_8` | MATCHEMPTY: `` (empty), `a*`, `a?`, `(?:)`, `(?=a)`, `\b`, `^`, `(a){0}` (set) vs `a`, `a+`, `[a]`, `(a\|b)` (clear). | [ ] |
| 149 | `pcre2_compile_8` | MAXLOOKBEHIND: 0 (`abc`), 1 (`\A`/`\b`/`\B` force 1), 2 (`(?<=ab)`), 4 (`(?<=a{1,4})`) — feeds `mb->allowemptypartial` and the UTF-check rewind at match time. | [ ] |
| 150 | `pcre2_code_copy_8` | copy of a pattern compiled with default tables: `PCRE2_DEREF_TABLES` **not** set, `tables` pointer shared; then `pcre2_code_free_8` both. Match with the copy. | [ ] |
| 151 | `pcre2_code_copy_with_tables_8` | copy of a pattern compiled with `pcre2_maketables_8` output: tables cloned into the block, `PCRE2_DEREF_TABLES` set; free the original tables first, then match with the copy. | [ ] |
| 152 | `pcre2_code_copy_8`, `pcre2_code_free_8` | copy of a deserialized code (`PCRE2_DEREF_TABLES` already set) — reference-count interaction on the shared table block. | [ ] |
| 153 | `pcre2_code_free_8` | `pcre2_code_free_8(NULL)` — legal no-op. | [ ] |

### 2. match — `pcre2_match_8` (+ `pcre2_match_data_create*_8`, `pcre2_get_*_8`, `pcre2_next_match_8`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 154 | `pcre2_match_8` | `/abc/` options 0, subject `xxabcxx` len 7, startoffset 0, `oveccount = 1`, mcontext NULL. Baseline: rc 1, ovector[0..1] = {2,5}, `startchar` 2, `leftchar` 2, `rightchar` 5, `mark` NULL, `matchedby = INTERPRETER`. | [ ] |
| 155 | `pcre2_match_8` | `subject == NULL`, `length == 0` ⇒ internal `null_str`; on nomatch `match_data->subject` must be set back to **NULL** (`original_subject`). | [ ] |
| 156 | `pcre2_match_8` | `length == PCRE2_ZERO_TERMINATED` on `abc\0def`: length resolved to 3 before the offset check. | [ ] |
| 157 | `pcre2_match_8` | `startoffset == length` on `/(?<=abc)/` with subject `abc`: legal, empty match at the end; and `/a/` at `startoffset == length` ⇒ NOMATCH. | [ ] |
| 158 | `pcre2_match_8` | `startoffset` mid-subject on `/(?<=xx)abc/` with `max_lookbehind 2`: `mb->check_subject` walks back 2 characters so the lookbehind may read before `startoffset`. | [ ] |
| 159 | `pcre2_match_8` | pattern-embedded `(*NOTEMPTY)` and `(*NOTEMPTY_ATSTART)` vs the same bits passed in `options`: identical behaviour, but `match_data->options` reflects `original_options` (pre-fold) only. | [ ] |
| 160 | `pcre2_match_8` | `PCRE2_NOTEMPTY` with `/a*/` on `bbb`: empty match at start rejected, bumpalong continues; contrast `PCRE2_NOTEMPTY_ATSTART` (only the start offset is forbidden). | [ ] |
| 161 | `pcre2_match_8` | `PCRE2_NOTEMPTY_ATSTART` with `startoffset 2` on `/a*/` subject `xxyy`: forbidden only at offset 2, so an empty match at 3 succeeds. | [ ] |
| 162 | `pcre2_match_8` | `PCRE2_NOTBOL` with `/^abc/` on `abc` (NOMATCH) and with `/^abc/m` on `abc\nabc` (first `^` blocked, second matches). | [ ] |
| 163 | `pcre2_match_8` | `PCRE2_NOTEOL` with `/abc$/` on `abc` (NOMATCH) and `/abc$/m` on `abc\nabc`. | [ ] |
| 164 | `pcre2_match_8` | compile `PCRE2_DOLLAR_ENDONLY`, `/abc$/` on `abc\n`: `$` behaves as `\z` ⇒ NOMATCH; without the option ⇒ match. | [ ] |
| 165 | `pcre2_match_8` | compile `PCRE2_MULTILINE\|PCRE2_ALT_CIRCUMFLEX`, `/^x/m` on `abc\n` with subject ending in the newline: `^` may match at `end_subject`. | [ ] |
| 166 | `pcre2_match_8` | compile `PCRE2_MATCH_UNSET_BACKREF`, `/(a)?\1b/` on `b`: unset group backref matches empty (`match_ref` returns 0 with length 0). | [ ] |
| 167 | `pcre2_match_8` | compile `PCRE2_MATCH_UNSET_BACKREF`, `/(a)?\1{2,3}b/` on `b`: the repeated-ref `Lmin == 0 \|\| MATCH_UNSET_BACKREF` `continue` path. | [ ] |
| 168 | `pcre2_match_8` | `PCRE2_ANCHORED` at match time on `/abc/` with subject `xabc`: NOMATCH (no bumpalong); compare with compile-time ANCHORED (either source works). | [ ] |
| 169 | `pcre2_match_8` | `PCRE2_ENDANCHORED` at match time on `/ab/` with `abc` (NOMATCH) and `ab` (match); and after `(*ACCEPT)` where the ENDANCHORED failure **hard-returns** instead of backtracking. | [ ] |
| 170 | `pcre2_match_8` | `mb->partial` selection: PARTIAL_SOFT only (1), PARTIAL_HARD only (2), **both** (HARD wins). Pattern `/abcd/`, subject `ab`. | [ ] |
| 171 | `pcre2_match_8` | `PCRE2_PARTIAL_SOFT` on `/abcd/` subject `ab`: `PCRE2_ERROR_PARTIAL`, ovector[0..1] = {0,2} only, **capture slots untouched**, `startchar` = partial start, `leftchar`/`rightchar` from `start_partial`/`end_subject`. | [ ] |
| 172 | `pcre2_match_8` | `PCRE2_PARTIAL_HARD` on `/abcd/` subject `abcd`: hard partial wins over the complete match at `\z`. | [ ] |
| 173 | `pcre2_match_8` | `PCRE2_PARTIAL_SOFT` with a pattern whose `max_lookbehind > 0` (`/(?<=abc)def/`) on subject `abc`: `mb->allowemptypartial` lets a zero-length partial be reported. | [ ] |
| 174 | `pcre2_match_8` | `PARTIAL_HARD` + `pcre2_set_newline(CRLF)` on `/./` with subject ending in a lone `\r`: the CRLF split partial special case in `OP_ANY`. | [ ] |
| 175 | `pcre2_match_8` | `PARTIAL_HARD` on `/abc\Z/` with subject `abc\r` under `NEWLINE_CRLF`: the `OP_EODN` CRLF-split partial arm. | [ ] |
| 176 | `pcre2_match_8` | `mb->partial != 0` disables the `minlength` and `req_cu` optimizations: `/abcdef/` PARTIAL_SOFT on a 2-byte subject still runs the attempt. | [ ] |
| 177 | `pcre2_match_8` | `PCRE2_NO_JIT` passed with no JIT built: accepted by the option mask, never tested ⇒ byte-identical result to options 0. | [ ] |
| 178 | `pcre2_match_8` | `PCRE2_COPY_MATCHED_SUBJECT` on a successful match, `length > 0`: `match_data->subject` points to a private copy, `PCRE2_MD_COPIED_SUBJECT` set; free the original subject then read via `pcre2_substring_get_bynumber_8`. | [ ] |
| 179 | `pcre2_match_8` | `PCRE2_COPY_MATCHED_SUBJECT` with `length == 0` (`//` on an empty subject): copy skipped, `subject == NULL`, but the flag **is** set. | [ ] |
| 180 | `pcre2_match_8` | reuse the same `match_data` for two `PCRE2_COPY_MATCHED_SUBJECT` matches: the previous copy is freed on entry and the flag cleared. | [ ] |
| 181 | `pcre2_match_8` | `PCRE2_COPY_MATCHED_SUBJECT` on a **partial** and on a **nomatch**: copy is *not* made; `subject == original_subject`. | [ ] |
| 182 | `pcre2_match_8` | `PCRE2_DISABLE_RECURSELOOP_CHECK` with `/(?1)()/`-style self recursion at the same position: loop suppressed, bounded by match/heap limit instead; without it ⇒ `PCRE2_ERROR_RECURSELOOP`. | [ ] |
| 183 | `pcre2_match_8`, `pcre2_set_offset_limit_8` | compile `PCRE2_USE_OFFSET_LIMIT`, `offset_limit = 4` on `/abc/` with subject `xxxxabc` (start at 4 — allowed, strict `>`), and `offset_limit = 3` (NOMATCH). | [ ] |
| 184 | `pcre2_match_8`, `pcre2_set_offset_limit_8` | compile `PCRE2_USE_OFFSET_LIMIT` + `PCRE2_NO_START_OPTIMIZE`: the `bumpalong_limit` check lives **outside** the START_OPTIMIZE guard so it still applies. | [ ] |
| 185 | `pcre2_match_8`, `pcre2_set_offset_limit_8` | mcontext with `offset_limit == PCRE2_UNSET` on a pattern *without* USE_OFFSET_LIMIT: legal (no error), `bumpalong_limit = true_end_subject`. | [ ] |
| 186 | `pcre2_match_8` | `mcontext == NULL` vs a default-constructed mcontext: allocator for heapframe growth comes from `re->memctl` in the first case, `mcontext->memctl` in the second (with a counting custom malloc via `pcre2_general_context_create_8`). | [ ] |
| 187 | `pcre2_match_8` | `PCRE2_NO_START_OPTIMIZE` at compile time, `/abc/` on `xxabcxx` with a callout: every bumpalong position fires a callout (no memchr skip, no minlength cut). | [ ] |
| 188 | `pcre2_match_8` | anchored + `has_first_cu` pre-check: compile `/abc/` `PCRE2_ANCHORED`, subject `xabc` (fails the single-position first-CU test) vs `abc`. | [ ] |
| 189 | `pcre2_match_8` | anchored + `start_bits` pre-check: compile `/[abc]x/` `PCRE2_ANCHORED`, subject `dx` vs `ax`. | [ ] |
| 190 | `pcre2_match_8` | unanchored **caseful** first-CU memchr: `/abc/` on a 4 KiB subject with the only `a` near the end; and with no `a` at all. | [ ] |
| 191 | `pcre2_match_8` | unanchored **caseless** first-CU dual-memchr + cache: `/(?i)abc/` on subjects containing (a) only `a`, (b) only `A`, (c) `A` before `a`, (d) `a` before `A`, (e) neither. | [ ] |
| 192 | `pcre2_match_8` | caseless dual-memchr **cache-hit** arms: `/(?i)ab/` on a subject with ≥3 failing candidate positions so `memchr_found_first_cu`/`_cu2` are reused across bumpalong iterations. | [ ] |
| 193 | `pcre2_match_8` | caseless first CU with `first_cu > 127` under `PCRE2_UCP` **without** UTF (8-bit-only `UCD_OTHERCASE` override) vs the same under UTF (flip-table value only). | [ ] |
| 194 | `pcre2_match_8` | `startline` bump: compile `/^abc/m`, subject `xxx\nabc\nabc` — the `WAS_NEWLINE` scan is skipped on the first iteration and used afterwards; plus the CR/LF fudge with `pcre2_set_newline(ANYCRLF)` and subject `x\r\nabc`. | [ ] |
| 195 | `pcre2_match_8` | `start_bits` bitmap scan (only reachable when `!has_first_cu && !startline`): `/[abc]x/` on a long subject with no `a`/`b`/`c`. | [ ] |
| 196 | `pcre2_match_8` | precedence of the three unanchored scans: a pattern with FIRSTSET, one with STARTLINE, one with FIRSTMAPSET — assert only one scan runs (`has_first_cu` > `startline` > `start_bits`). | [ ] |
| 197 | `pcre2_match_8` | `re->minlength` cut: `/abcdef/` (minlength 6) with subject len 5 ⇒ immediate NOMATCH without any attempt; and minlength measured in **code units** vs UTF characters (`/\x{100}\x{100}\x{100}/` on a 4-byte subject). | [ ] |
| 198 | `pcre2_match_8` | `req_cu` caseful check: `/a.*z/` on a 100-byte subject with no `z`; `check_length < REQ_CU_MAX` (5000) vs `< REQ_CU_MAX*1000` unanchored (a 20 000-byte subject) vs anchored (only the 5000 window). | [ ] |
| 199 | `pcre2_match_8` | `req_cu` caseless: `/(?i)a.*z/` with only `Z` present (second memchr) and with neither; and `has_first_cu` skipping one code unit before the search. | [ ] |
| 200 | `pcre2_match_8` | `req_cu_ptr` monotonic cache across bumpalong: `/a.*z/` on `aaaaaaaaz` — after the first successful req-CU find, later iterations skip the search (`p > req_cu_ptr`). | [ ] |
| 201 | `pcre2_match_8` | compile `PCRE2_FIRSTLINE`, `/abc/` with subject `xx\nabc`: the temporary `end_subject` clamp ⇒ NOMATCH; subject `abcxx\nyy` ⇒ match. Repeat with UTF (separate clamp loop). | [ ] |
| 202 | `pcre2_match_8` | `PCRE2_FIRSTLINE` + `PARTIAL_SOFT`: the `mb->partial == 0 && start_match >= mb->end_subject` escape hatch compares against the **true** end, so the attempt at the clamped end still runs. | [ ] |
| 203 | `pcre2_match_8` | `PCRE2_FIRSTLINE` and `IS_NEWLINE(start_match)` after a failed attempt ⇒ bumpalong stops. Subject `ab\ncd` with `/cd/`. | [ ] |
| 204 | `pcre2_match_8` | CRLF bumpalong skip: `/x/` with `pcre2_set_newline(CRLF)` on subject `a\r\nx`, pattern without literal `\r`/`\n` (HASCRORLF clear) — `start_match++` skips the LF. Then repeat with `/\rx/` (HASCRORLF set ⇒ no skip), with NEWLINE_ANY, ANYCRLF, and LF (no skip). | [ ] |
| 205 | `pcre2_match_8` | bumpalong step in UTF: `/x/` `PCRE2_UTF` on `\xC3\xA9\xC3\xA9x` — `ACROSSCHAR` advances whole characters. | [ ] |
| 206 | `pcre2_match_8` | position exactly at `end_subject` is tried (`start_match > end_subject` is the break, not `>=`): `/(?<=abc)/` on `abc` unanchored. | [ ] |
| 207 | `pcre2_match_8` | `PCRE2_UTF` + valid multi-byte subject, `PCRE2_NO_UTF_CHECK` **clear**: `_pcre2_valid_utf_8` runs over `check_subject`..end. | [ ] |
| 208 | `pcre2_match_8` | `PCRE2_UTF\|PCRE2_NO_UTF_CHECK`, valid subject: check skipped; `mb->check_subject == subject` so lookbehind reach differs from row 207 (`/(?<=a)b/` with `startoffset` 1 on `ab`). | [ ] |
| 209 | `pcre2_match_8` | compile `PCRE2_MATCH_INVALID_UTF`, match option `PCRE2_NO_UTF_CHECK` set: `allow_invalid` **overrides** NO_UTF_CHECK, the check always runs. | [ ] |
| 210 | `pcre2_match_8` | compile `PCRE2_MATCH_INVALID_UTF`, subject `abc\xFFdef`, `/def/`: fragment loop — first fragment gets `fragment_options = PCRE2_NOTEOL`, the last gets `PCRE2_NOTBOL`; match found in the second fragment. | [ ] |
| 211 | `pcre2_match_8` | compile `PCRE2_MATCH_INVALID_UTF`, subject with **two** bad bytes `a\xFFb\xFFc`: middle fragment gets `NOTBOL\|NOTEOL`; `FRAGMENT_RESTART` clears `hitend` and both memchr caches. | [ ] |
| 212 | `pcre2_match_8` | compile `PCRE2_MATCH_INVALID_UTF`, `startoffset` mid-character: `skipped_bad_start` advances past continuation bytes and **suppresses** the `max_lookbehind` rewind. | [ ] |
| 213 | `pcre2_match_8` | compile `PCRE2_MATCH_INVALID_UTF`, anchors across fragments: `/\A./`, `/\z/`, `/\G./` with `startoffset > 0` on a subject with a bad byte — `\A` uses `mb->start_subject`, `\z` uses `true_end_subject`, `\G` uses the *original* start offset. | [ ] |
| 214 | `pcre2_match_8` | compile `PCRE2_MATCH_INVALID_UTF`, `PARTIAL_SOFT`: a partial found in a non-final fragment is discarded and matching continues. | [ ] |
| 215 | `pcre2_match_data_create_8`, `pcre2_get_ovector_count_8`, `pcre2_get_ovector_pointer_8`, `pcre2_get_match_data_size_8`, `pcre2_match_data_free_8` | `pcre2_match_data_create_8(0, NULL)` ⇒ clamped to 1 pair; `create(70000, NULL)` ⇒ clamped to `UINT16_MAX`; `create(4, gcontext)` with a counting allocator. Check `pcre2_get_ovector_count_8`, that `pcre2_get_ovector_pointer_8` points at `offsetof(pcre2_real_match_data, ovector)`, and `pcre2_get_match_data_size_8` = `offsetof(…, ovector) + 2*oveccount*sizeof(PCRE2_SIZE)`. Free with `pcre2_match_data_free_8`, including `pcre2_match_data_free_8(NULL)` and a block whose `heapframes` and `PCRE2_MD_COPIED_SUBJECT` copy both need freeing. | [ ] |
| 216 | `pcre2_match_8`, `pcre2_match_data_create_from_pattern_8` | `/(a)(b)(c)/` ⇒ 4 pairs; NULL gcontext ⇒ allocator taken from the code; explicit gcontext with counting malloc. | [ ] |
| 217 | `pcre2_match_8` | `oveccount == 1` on `/(a)(b)/` matching `ab`: rc 0 ("ovector too small"), only pair 0 written, `memcpy` of 0 bytes, trailing `PCRE2_UNSET` loop a no-op. | [ ] |
| 218 | `pcre2_match_8` | `oveccount` exactly `top_bracket+1` on `/(a)(b)/`: rc 3, all pairs set. | [ ] |
| 219 | `pcre2_match_8` | `oveccount` oversized (10) on `/(a)(b)/`: rc 3; slots 3..9 are **left untouched** (stale) — pre-fill them and assert they survive. | [ ] |
| 220 | `pcre2_match_8` | `oveccount` between: `/(a)(b)(c)/` with `oveccount 3` ⇒ truncation; `end_offset_top >= 2*oveccount` ⇒ rc 0. | [ ] |
| 221 | `pcre2_match_8` | non-participating group below `Foffset_top`: `/(a)\|(b)/` on `b` ⇒ ovector[2..3] `PCRE2_UNSET` (from the 0xff memset), ovector[4..5] set. | [ ] |
| 222 | `pcre2_match_8` | groups above `Foffset_top`: `/(a)(b)?/` on `a` with `oveccount 3` ⇒ ovector[4..5] explicitly `PCRE2_UNSET` by the `while (--i …)` loop. | [ ] |
| 223 | `pcre2_match_8` | rc 0 driven by `end_offset_top`, not `top_bracket`: `/(a)\|(b)\|(c)/` with `oveccount 2` on `a` ⇒ positive rc even though top_bracket is 3. | [ ] |
| 224 | `pcre2_match_8`, `pcre2_get_startchar_8` | `\K` moving the match start: `/ab\Kcd/` on `abcd` ⇒ ovector[0] = 2 but `startchar` = 0 (the bumpalong position). | [ ] |
| 225 | `pcre2_match_8`, `pcre2_get_mark_8` | `(*MARK:x)` reached on a **successful** match ⇒ `mark == "x"`; and on NOMATCH ⇒ `mark == mb->nomatch_mark`; and on a hard error the mark is also overwritten. | [ ] |
| 226 | `pcre2_match_8`, `pcre2_get_mark_8` | `(*SKIP:x)` does **not** set `nomatch_mark` (unlike `(*MARK:)`, `(*COMMIT:)`, `(*PRUNE:)`, `(*THEN:)`) — one row per verb-with-arg. | [ ] |
| 227 | `pcre2_match_8`, `pcre2_get_match_data_heapframes_size_8` | reuse one `match_data` across a 0-capture pattern then a 200-capture pattern (heapframes grow) then back (no realloc): assert `heapframes_size` monotonic. | [ ] |
| 228 | `pcre2_match_8`, `pcre2_set_heap_limit_8` | `heap_limit` large enough for the initial `max(frame_size*10, START_FRAMES_SIZE)`; and `heap_limit = 19` KiB with a small pattern (vector clamped to 19456 instead of 20480). | [ ] |
| 229 | `pcre2_match_8`, `pcre2_set_heap_limit_8` | growth inside `match()`: deep backtracking pattern where doubling succeeds; where doubling is clipped by `heap_limit` but still fits ≥1 frame; and reuse of a match_data whose vector is already large enough. | [ ] |
| 230 | `pcre2_match_8`, `pcre2_set_match_limit_8` | `match_limit` from context only; from `(*LIMIT_MATCH=n)` only; both with the pattern smaller; both with the context smaller (min wins); neither (10000000). Note the counter resets per bumpalong position. | [ ] |
| 231 | `pcre2_match_8`, `pcre2_set_depth_limit_8` | same 5-way matrix for `depth_limit` / `(*LIMIT_DEPTH=n)`; also `pcre2_set_recursion_limit_8` as the synonym. | [ ] |
| 232 | `pcre2_match_8`, `pcre2_set_heap_limit_8` | same 5-way matrix for `heap_limit` / `(*LIMIT_HEAP=n)`. | [ ] |
| 233 | `pcre2_match_8`, `pcre2_set_recursion_memory_management_8` | obsolete setter is a no-op: call it, then match ⇒ identical results. | [ ] |
| 234 | `pcre2_match_8` | `match_ref` caseless UTF/UCP path: `/(?i)(\x{c5})\1/` UTF on the two case forms; and the caseless-restrict variant returning -1 for `*pp < 128`; and the Turkish-casing variant with `REFI_FLAG_TURKISH_CASING`. | [ ] |
| 235 | `pcre2_match_8` | `match_ref` caseful split: `/(abc)\1/` with `mb->partial == 0` (bulk `memcmp`) vs `PARTIAL_SOFT` (unit-by-unit loop returning >0 for a partial). | [ ] |
| 236 | `pcre2_match_8` | `match_ref` returning **partial** (>0): `/(abc)\1/` PARTIAL_SOFT on `abcab`; and the maximizing-loop variant that does **not** advance `Feptr`. | [ ] |
| 237 | `pcre2_match_8` | `OP_DNREF`/`OP_DNREFI` with DUPNAMES: `/(?<a>x)\|(?<a>y)\k<a>/` where the first candidate is unset and the second set; and where none is set. | [ ] |
| 238 | `pcre2_match_8` | backref repeat forms: `/(a)\1*/`, `/(a)\1+/`, `/(a)\1?/`, `/(a)\1{2,3}/`, `/(a)\1{2,}/`, `/(a)\1*+/` — CRSTAR/CRPLUS/CRQUERY/CRRANGE/CRMINRANGE/CRPOSSTAR. | [ ] |
| 239 | `pcre2_match_8` | zero-length set group backref `continue` guard: `/()\1*x/` on `x`. | [ ] |
| 240 | `pcre2_match_8` | backref maximizing `samelengths` fast path vs the caseless-UTF re-scan slow path: `/(?i)(\x{23a})\1+/` UTF where iteration lengths differ (U+023A vs U+2C65). | [ ] |
| 241 | `pcre2_match_8` | `OP_RECURSE` whole-pattern (group 0): `/\((?:[^()]\|(?R))*\)/` on nested parentheses. | [ ] |
| 242 | `pcre2_match_8` | `OP_RECURSE` group recursion `/(a(?1)?b)/` on `aabb`; captures are **not** propagated out (`OP_CLOSE` skipped when `Fcurrent_recurse != RECURSE_UNSET`). | [ ] |
| 243 | `pcre2_match_8` | recurse-loop check triggers: same group number, same subject position, unchanged `last_used_ptr` ⇒ `PCRE2_ERROR_RECURSELOOP`; same number but position advanced ⇒ no error; different number ⇒ no error. | [ ] |
| 244 | `pcre2_match_8` | recursion with an argument list `/(a)(b)(?1(2))/`: `recurse_update_offsets` selectively preserves the listed groups instead of a straight `memcpy`. | [ ] |
| 245 | `pcre2_match_8` | `(*ACCEPT)` inside a recursion: walks back to the `GF_RECURSE` frame and resumes after the recurse opcode. | [ ] |
| 246 | `pcre2_match_8` | verb containment in a recursion: `(*COMMIT)`/`(*PRUNE)`/`(*THEN)` inside `(?1)` — `mb->verb_current_recurse` decides conversion to NOMATCH vs propagation. | [ ] |
| 247 | `pcre2_match_8` | atomic group `OP_ONCE`: `/(?>a+)ab/` on `aaab` ⇒ NOMATCH (alternatives discarded via `Fback_frame` and the ket branch-advance). | [ ] |
| 248 | `pcre2_match_8` | possessive group family: `/(?:a\|ab)++c/`, `/(a)++/`, `/(?:a)?+/` — `OP_BRAPOS`, `OP_CBRAPOS`, `OP_SBRAPOS`, `OP_BRAPOSZERO`, `OP_KETRPOS`, the empty-iteration break, and `Lmatched_once \|\| Lzero_allowed`. | [ ] |
| 249 | `pcre2_match_8` | possessive quantifiers with **no** backtracking frame (`REPTYPE_POS`) across all families: `a++`, `[ab]++`, `\d++`, `\p{L}++`, `[\x{100}]++` (XCLASS), `(?[a&&a])++` (ECLASS), `\1++`. | [ ] |
| 250 | `pcre2_match_8` | fixed lookbehind `OP_REVERSE`: `/(?<=abc)d/` on `abcd` with `startoffset 3`, non-UTF (floor `mb->start_subject`) and UTF (floor `mb->check_subject`) — the two different floors. | [ ] |
| 251 | `pcre2_match_8` | variable lookbehind `OP_VREVERSE`: `/(?<=ab{1,3})c/` on `abbbc`; the retry loop, the UTF start-clamp shrink, and the non-UTF `available = min(…, 65535)` clamp. | [ ] |
| 252 | `pcre2_match_8` | variable-lookbehind end-point verification in all 4 sites: `(?<=a\|bb)` as a conditional assertion, as `OP_ASSERTBACK`, as `OP_ASSERTBACK_NA` (`(*naplb:)`), and as `OP_ASSERTBACK_NOT`. | [ ] |
| 253 | `pcre2_match_8` | `\X` single (`OP_EXTUNI`): `/\X/` UTF on `a`+U+0301, on `\r\n`, on `U+1F468 U+200D U+1F469`, on 2 and 3 regional indicators, on Hangul `L+V`. | [ ] |
| 254 | `pcre2_match_8` | `\X` min-repeat and max-repeat with backtracking: `/\X{2,4}z/` UTF on a mix of clusters — the max-repeat re-derives breaks with `UCD_GRAPHBREAK` + `_pcre2_ucp_gbtable_8` and uses `Feptr <= Lstart_eptr`. | [ ] |
| 255 | `pcre2_match_8` | `(*script_run:)`: `/(*sr:\w+)/` UTF+UCP on all-Latin (match), Latin+Cyrillic (NOMATCH), Han+Hiragana (match), mixed digit sets `1٢` (NOMATCH). | [ ] |
| 256 | `pcre2_match_8` | `OP_XCLASS` single / min-repeat / max-repeat: `/[\x{100}-\x{200}]/`, `/[\x{100}]{2,4}/` UTF, with `\C` in the pattern so `Feptr-- <= Lstart_eptr` matters. | [ ] |
| 257 | `pcre2_match_8` | `OP_ECLASS` single / min / max repeat: `/(?[[\p{L}] - [a-z]])+/` UTF. | [ ] |
| 258 | `pcre2_match_8` | `OP_CLASS` vs `OP_NCLASS` `>255` handling: `/[^x]/` on U+0100 with `PCRE2_UTF` (NCLASS matches) vs non-UTF (identical because the `>255` test is compiled out). | [ ] |
| 259 | `pcre2_match_8`, `pcre2_set_callout_8` | numeric callout `(?C1)`: assert `version == 2`, `callout_number`, `capture_top == Foffset_top/2+1`, `capture_last`, `pattern_position`, `next_item_length`, `offset_vector[0]`/`[1]` are `PCRE2_UNSET` **inside** the callout and restored afterwards. | [ ] |
| 260 | `pcre2_match_8`, `pcre2_set_callout_8` | string callout `(?C"str")` with LINK_SIZE=2: `callout_string_offset`, `callout_string_length`, `callout_string` pointer. | [ ] |
| 261 | `pcre2_match_8`, `pcre2_set_callout_8` | callout return values: 0 (continue), >0 (`MATCH_NOMATCH`), <0 (propagated, e.g. `PCRE2_ERROR_CALLOUT`). | [ ] |
| 262 | `pcre2_match_8`, `pcre2_set_callout_8` | `callout_flags`: `PCRE2_CALLOUT_STARTMATCH` on each bumpalong, `PCRE2_CALLOUT_BACKTRACK` after a backtrack, cleared after each callout. Pattern `/a(?C1)b/` on `axab`. | [ ] |
| 263 | `pcre2_match_8` | pattern with callouts but `mcontext->callout == NULL` (and mcontext NULL): `do_callout` returns 0 immediately after setting the length — match proceeds. | [ ] |
| 264 | `pcre2_match_8`, `pcre2_set_callout_8` | `PCRE2_AUTO_CALLOUT` compile + callout function: full callout log for `/ab/` on `ab`; and `cb.subject_length` being the **fragment** length under `MATCH_INVALID_UTF`. | [ ] |
| 265 | `pcre2_match_8`, `pcre2_set_callout_8` | conditional-group callout: `PCRE2_AUTO_CALLOUT` with `/(?(?=a)b\|c)/` — the `Llength -= length` adjustment. | [ ] |
| 266 | `pcre2_match_8` | `(*COMMIT)`: `/a(*COMMIT)b/` on `axab` ⇒ NOMATCH with **no** bumpalong. | [ ] |
| 267 | `pcre2_match_8` | `(*PRUNE)`: `/a(*PRUNE)b/` on `axab` ⇒ bumpalong advances one character (behaves like NOMATCH at top level). | [ ] |
| 268 | `pcre2_match_8` | `(*SKIP)`: `/a(*SKIP)b/` on `aaxab` ⇒ `new_start_match = mb->verb_skip_ptr` when it is ahead, else degrade to NOMATCH. | [ ] |
| 269 | `pcre2_match_8` | `(*SKIP:name)` + `(*MARK:name)`: `/(*MARK:m)a(*SKIP:m)b/` — `MATCH_SKIP_ARG`, retry at the **same** position with `ignore_skip_arg = skip_arg_count`, and the `skip_arg_count <= ignore_skip_arg` verb-skip path. | [ ] |
| 270 | `pcre2_match_8` | `(*THEN)`: `/(a(*THEN)b\|ac)/` on `ac` (converted to NOMATCH by the enclosing group) vs `/a(*THEN)b/` at top level (acts like PRUNE) vs `(?=a(*THEN)b)` (swallowed by the assertion). | [ ] |
| 271 | `pcre2_match_8` | `(*ACCEPT)` at top level and inside an assertion: `assert_accept_frame` capture/offset_top/mark copy-out. | [ ] |
| 272 | `pcre2_match_8` | anchors: `\A` (`Feptr != mb->start_subject`) with `startoffset > 0`; `\G` (`start_subject + start_offset`) with `startoffset > 0`; `\Z` before a final newline; `\z` at true end. Subject `xxabc\n`. | [ ] |
| 273 | `pcre2_match_8` | `\K` validity: `/(?<=a\Kb)/`-style pushing `Fstart_match` before `startoffset` — legal only with `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK` (`mb->allowlookaroundbsk`). | [ ] |
| 274 | `pcre2_match_8` | `\b`/`\B`/UCP variants: `/\bx/` with `startoffset 1` (the "previous char" floor is `mb->check_subject`, not `start_subject`); non-UCP `ctype_word` vs `OP_UCP_WORD_BOUNDARY` (`ucp_L`/`ucp_N`/`Mn`/`Pc`) on U+00E9 and U+203F. | [ ] |
| 275 | `pcre2_match_8` | `.` under all 6 newline conventions × {DOTALL, not}: subject `a\rb\nc\r\nd` — `OP_ANY` newline exclusion vs `OP_ALLANY`. | [ ] |
| 276 | `pcre2_match_8` | `^`/`$` multiline under all 6 newline conventions on `a\rb\nc\r\nd\x0be`: `OP_CIRCM` / `OP_DOLLM` and the `WAS_NEWLINE`/`IS_NEWLINE` back-write of `mb->nllen` for ANY/ANYCRLF. | [ ] |
| 277 | `pcre2_match_8` | `\R` under `PCRE2_BSR_UNICODE` vs `PCRE2_BSR_ANYCRLF` on subject `\n\r\r\n\x0b\x0c\xc2\x85\xe2\x80\xa8\xe2\x80\xa9` with and without `PCRE2_UTF`; single, min-repeat and max-repeat forms; the CRLF-atomic backtrack fix-ups. | [ ] |
| 278 | `pcre2_match_8` | 8-bit non-UTF `\R` max-repeat: 0x2028/0x2029 are **not** tested in the non-UTF max-repeat loop (a real behavioural difference from the single-item case). | [ ] |
| 279 | `pcre2_match_8` | `\C` (`OP_ANYBYTE`) in UTF: `/\C\C/` on a 2-byte character (lands mid-character); and quantified `/\C{2}/`; and non-UTF `\C` compiled as `OP_ALLANY`. | [ ] |
| 280 | `pcre2_match_8` | `\h`/`\H`/`\v`/`\V` single and repeat, UTF and non-UTF (the `HSPACE_MULTIBYTE_CASES` arm is compiled out of the non-UTF max-repeat path in 8-bit). | [ ] |
| 281 | `pcre2_match_8` | `\p`/`\P` for each of the 13 `PT_*` types, in single, min-repeat and max-repeat form — with `PCRE2_UCP` turning `\d`/`\w`/`\s` into these opcodes. | [ ] |
| 282 | `pcre2_match_8` | assertions: `(?=)`/`(?!)`/`(?<=)`/`(?<!)`/`(*napla:)`/`(*naplb:)`; the negative form's 4-way switch where `MATCH_COMMIT`/`MATCH_SKIP`/`MATCH_PRUNE` mean *success*. | [ ] |
| 283 | `pcre2_match_8` | `(*scs:1)`/`(*scan_substring:<n>)`: group set (scan inside the captured substring, `NOTEOL` cleared so `$` works) vs group unset (NOMATCH) vs a DUPNAMES name list. | [ ] |
| 284 | `pcre2_match_8` | conditions at match time: `OP_RREF` `RREF_ANY` and specific, `OP_DNRREF`, `OP_CREF` set/unset, `OP_DNCREF`, `OP_FALSE`, `OP_TRUE`, assertion condition. | [ ] |
| 285 | `pcre2_match_8` | infinite-loop guard at `OP_KET`: `/(a*)*b/` on `aaac` — `Fop != OP_KET && Feptr != P->eptr`. | [ ] |
| 286 | `pcre2_match_8` | `OP_BRA` fast path: the same pattern with and without a `(*THEN)` anywhere (`mb->hasthen`) ⇒ different frame counts ⇒ different `match_limit` consumption. | [ ] |
| 287 | `pcre2_match_8` | error return leaves `match_data->subject == NULL` and stale `subject_length`/`start_offset`/`ovector`: force `PCRE2_ERROR_MATCHLIMIT` with `match_limit = 1`. | [ ] |
| 288 | `pcre2_next_match_8` | non-empty match: `*pstart_offset = ovector[1]`, `*poptions = 0`. Driver: `/a/` global loop over `aaa`. | [ ] |
| 289 | `pcre2_next_match_8` | empty match not at end: `*pstart_offset = ovector[1]` (unchanged), `*poptions = PCRE2_NOTEMPTY_ATSTART`; empty match at end ⇒ FALSE. Pattern `/a*/` on `xax`. | [ ] |
| 290 | `pcre2_next_match_8` | `\K`-in-lookaround case (`ovector[0] != start_offset && ovector[1] == start_offset`) with `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK`: `do_bumpalong` is used — one row per branch of `do_bumpalong` (CRLF-atomic skip under CRLF/ANY/ANYCRLF, UTF `FORWARDCHARTEST`, plain `+1`). | [ ] |
| 291 | `pcre2_next_match_8` | `match_data->rc < 0` (NOMATCH or an error) ⇒ FALSE, outputs untouched. | [ ] |
| 292 | `pcre2_match_8`, `pcre2_next_match_8` | full global-iteration driver: `/a*/` on `\r\naa\r\n` with `pcre2_set_newline(CRLF)` and with LF — assert the sequence of `(start_offset, options)` pairs. | [ ] |
| 293 | `pcre2_jit_match_8` | no-JIT stub: any valid code/subject/match_data ⇒ `PCRE2_ERROR_JIT_BADOPTION`, and `match_data->rc` is set to it. | [ ] |

### 3. dfa_match — `pcre2_dfa_match_8`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 294 | `pcre2_dfa_match_8` | `/abc/` options 0, subject `xxabcxx`, startoffset 0, `oveccount 1`, `wscount 20` (the minimum ⇒ 3 states/vector), mcontext NULL. Baseline: rc 1, `matchedby = DFA_INTERPRETER`, `mark == NULL` always. | [ ] |
| 295 | `pcre2_dfa_match_8` | `subject == NULL, length == 0` ⇒ `null_str`; `length == PCRE2_ZERO_TERMINATED`; `startoffset == length`. | [ ] |
| 296 | `pcre2_dfa_match_8` | `wscount` sizing: 20 (3 states), 1000 (166 states), and `20*(1+top_bracket)` on a 5-group pattern — the capacity formula is `floor((wscount-2)/6)` per vector. | [ ] |
| 297 | `pcre2_dfa_match_8` | one-start/many-ends ovector: `/a\|ab\|abc/` on `abc` with `oveccount` 1 / 2 / 3 / 5 — ends ordered **longest first**, `ovector[2k]` always equal, rc latched to 0 once `match_count*2 > offsetcount`. | [ ] |
| 298 | `pcre2_dfa_match_8` | `oveccount == 1` with 3 distinct ends ⇒ rc 0 with the longest end kept; and `oveccount` exactly equal to the number of ends ⇒ positive rc. | [ ] |
| 299 | `pcre2_dfa_match_8` | `PCRE2_DFA_SHORTEST` on `/a\|ab\|abc/` over `abc`: returns at the first acceptance ⇒ rc 1 with the **shortest** end, and the ENDANCHORED post-check is bypassed. | [ ] |
| 300 | `pcre2_dfa_match_8` | `PCRE2_ENDANCHORED` on `/a\|ab/` over `ab`: the post-check tests the DFA scan position `ptr`, not each recorded end — shorter, non-end-anchored ends can still appear in `ovector[2..]`. | [ ] |
| 301 | `pcre2_dfa_match_8` | `PCRE2_DFA_RESTART` two-call sequence: call 1 with `PARTIAL_SOFT` on `/abcd/` subject `ab` (rc `PCRE2_ERROR_PARTIAL`), call 2 with `DFA_RESTART` at `ovector[1]` on `cd`, **same workspace pointer and same `wscount`**. Restart forces `anchored`, kills `firstline` and all start optimizations. | [ ] |
| 302 | `pcre2_dfa_match_8` | `PCRE2_DFA_RESTART` with a valid `workspace[0] ∈ {0,1}` and `workspace[1]` at both the lower bound 1 and the upper bound `(wscount-2)/3` — exercise both halves (`workspace[0]` 0 ⇒ the `memcpy` back-fill path). | [ ] |
| 303 | `pcre2_dfa_match_8` | `PCRE2_NOTBOL` / `PCRE2_NOTEOL` / `PCRE2_NOTEMPTY` / `PCRE2_NOTEMPTY_ATSTART` on `/^a*$/m` over `\na\n`: note DFA `OP_DOLLM` still matches internal newlines under NOTEOL, and `^` compares against absolute offset 0 (`start_subject`), not `start_offset`. | [ ] |
| 304 | `pcre2_dfa_match_8` | `PCRE2_PARTIAL_SOFT` vs `PCRE2_PARTIAL_HARD` on `/abcd/` subject `ab`: `could_continue`, `partial_newline`, `ptr > mb->start_used_ptr \|\| mb->allowemptypartial`; HARD makes `\z`/`\Z` return `PCRE2_ERROR_PARTIAL` immediately even after a complete match was recorded. | [ ] |
| 305 | `pcre2_dfa_match_8` | `PCRE2_ERROR_PARTIAL` ovector contents: `{start_match, end_subject}` with `oveccount > 0`, and **nothing written** with `oveccount == 0` semantics (`offsetcount == 0` via a 1-pair block never happens through the API — use the `rc == 0` latch instead). | [ ] |
| 306 | `pcre2_dfa_match_8` | `PCRE2_COPY_MATCHED_SUBJECT` applied only on `rc >= 0` (**not** on `PCRE2_ERROR_PARTIAL`); `length == 0` ⇒ `subject = NULL`; reuse of the same match_data frees the previous copy. | [ ] |
| 307 | `pcre2_dfa_match_8` | compile `PCRE2_UTF`, valid multi-byte subject, with and without `PCRE2_NO_UTF_CHECK`; startoffset on a character boundary. | [ ] |
| 308 | `pcre2_dfa_match_8` | compile `PCRE2_FIRSTLINE`: the `end_subject = t` fudge, its restore, and the `firstline && IS_NEWLINE(start_match)` bumpalong terminator. | [ ] |
| 309 | `pcre2_dfa_match_8`, `pcre2_set_offset_limit_8` | compile `PCRE2_USE_OFFSET_LIMIT`, limit at the exact match start (allowed, strict `>`) and one below. | [ ] |
| 310 | `pcre2_dfa_match_8` | DFA start optimizations, one row per arm: anchored first-CU/bitmap pre-check; unanchored caseless dual-memchr with the `memchr_found_first_cu*` caches; caseful memchr; `startline` scan + CR/LF fudge; `start_bits` bitmap; `minlength` cut (`goto NOMATCH_EXIT`); `req_cu` window 5000 anchored vs 5 000 000 unanchored. | [ ] |
| 311 | `pcre2_dfa_match_8` | compile `PCRE2_NO_START_OPTIMIZE` (or `(*NO_START_OPT)`): whole optimization block skipped; also `PCRE2_DFA_RESTART` skipping it independently. | [ ] |
| 312 | `pcre2_dfa_match_8` | DFA-supported constructs one row each: atomic `(?>a+)b`, possessive `(?:a)++b` / `(a)++`, single-char possessive `a++`, fixed lookahead `(?=a)`, fixed lookbehind `(?<=ab)`, negative lookaround, recursion `(?R)` / `(?1)`, `(*FAIL)`, `\X`, `\p{L}`, `OP_XCLASS`, `OP_ECLASS`, UCP `\b`, callouts. | [ ] |
| 313 | `pcre2_dfa_match_8` | DFA lookbehind setup: multi-branch lookbehind `(?<=ab\|c)d` where `max_back` is the maximum over branches, UTF stepping back character-by-character, and `gone_back` clamped at the subject start (`startoffset` 0 vs 3). | [ ] |
| 314 | `pcre2_dfa_match_8`, `pcre2_set_callout_8` | DFA callout block: `version == 2`, `capture_top == 1`, `capture_last == 0`, `mark == NULL`; return 0 (continue), >0 (thread dies), <0 (abandon match). | [ ] |
| 315 | `pcre2_dfa_match_8`, `pcre2_set_match_limit_8`, `pcre2_set_depth_limit_8` | `match_limit` counts **total** `internal_dfa_match` invocations (reset once per call, not per bumpalong): set it just above the number of start positions for `/a/` on a 10-byte subject. `depth_limit` bounds the nesting depth at `limit+1`: `(?>(?>(?>a)))` with limit 2 and 3. Pattern `(*LIMIT_MATCH=n)` lowers it. | [ ] |
| 316 | `pcre2_dfa_match_8`, `pcre2_set_heap_limit_8` | RWS growth: a pattern nesting 8 assertions (>7 fit in the 7676-int base block) so `more_workspace` mallocs 60 KiB; `heap_limit` set to exactly the clamp threshold (≥4 KiB for assertions, ≥12 KiB for recursion) — and the cached `rws->next` reuse on a second nesting. | [ ] |
| 317 | `pcre2_dfa_match_8` | nested calls always get `wscount = 1000` (166 states) regardless of the caller's `wscount`: a pattern whose assertion body needs many simultaneous states, with a huge outer workspace. | [ ] |
| 318 | `pcre2_dfa_match_8` | duplicate-state suppression (`offset` and `count` equal): `/(a*)*b/` on `aaac` terminates instead of looping. | [ ] |
| 319 | `pcre2_dfa_match_8` | newline handling in the DFA under all 6 conventions for `.`, `$`, `$` multiline, `^` multiline, `\Z`, `\z` on subject `a\rb\nc\r\nd\x0be` — note CRLF makes `.` exclude only a CR *followed by* LF. | [ ] |
| 320 | `pcre2_dfa_match_8` | `\R` in the DFA: `PCRE2_BSR_UNICODE` vs `PCRE2_BSR_ANYCRLF`, single and quantified (`OP_ANYNL_EXTRA` variants), CR+LF consuming 2 units; independent of `newline_convention`. | [ ] |
| 321 | `pcre2_dfa_match_8` | `mb->start_used_ptr`/`last_used_ptr` ⇒ `match_data->leftchar`/`rightchar`, pushed back by lookbehind and by `\b`: `/(?<=ab)\bc/` with `startoffset 2`. Note NOMATCH does **not** set these. | [ ] |

### 4. substitute — `pcre2_substitute_8`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 322 | `pcre2_substitute_8` | baseline: `/b/`, subject `abc` len 3, startoffset 0, options 0, `match_data` NULL (internal block created), mcontext NULL, replacement `X` len 1, buffer 16, `*blength = 16`. rc 1, output `aXc`, `*blength = 3`, trailing NUL written. | [ ] |
| 323 | `pcre2_substitute_8` | `subject == NULL, length == 0`; `length == PCRE2_ZERO_TERMINATED`; `replacement == NULL, rlength == 0`; `rlength == PCRE2_ZERO_TERMINATED`; `rlength == 0` (deletion); replacement containing embedded NULs. | [ ] |
| 324 | `pcre2_substitute_8` | `startoffset` 0 / mid / `== length`: prefix `subject[0..startoffset)` copied verbatim unless `REPLACEMENT_ONLY`. | [ ] |
| 325 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_GLOBAL` on `/a/` over `aaa`; and on `/a*/` over `xax` (empty-match retry via `pcre2_next_match` with `NOTEMPTY_ATSTART`, and the `ovector[0]-start_offset` gap copy). | [ ] |
| 326 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_GLOBAL` where the `NOTEMPTY_ATSTART` retry returns a match starting **later** than `start_offset` (unanchored retry) — assert the gap is copied. | [ ] |
| 327 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_GLOBAL\|PCRE2_ANCHORED`: every iteration must match exactly at the current offset (no ANCHORED is ever added internally). | [ ] |
| 328 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_LITERAL` (takes precedence over EXTENDED) with a replacement containing `$` and `\`: single `CHECKMEMCPY` per match. | [ ] |
| 329 | `pcre2_substitute_8` | non-EXTENDED `$` forms: `$$`, `$&`, `` $` ``, `$'`, `$_`, `$1`, `$12`, `${1}`, `$name`, `${name}`, `$<name>`, `$+`, `$*MARK`, `${*MARK}` — one row's worth of sub-cases, on `/(?<n>b)(c)?/` over `abc` with `(*MARK:m)`. | [ ] |
| 330 | `pcre2_substitute_8` | `$*MARK` / `${*MARK}` with **no** mark set ⇒ emit nothing, no error; with a mark containing an embedded NUL (`fraglength = mark[-1]`). | [ ] |
| 331 | `pcre2_substitute_8` | `$+` matrix: `top_bracket == 0` with and without `UNKNOWN_UNSET`; `oveccount < top_bracket+1` ⇒ `PCRE2_ERROR_UNAVAILABLE`; last-set-group scan; no group set with `UNSET_EMPTY`. | [ ] |
| 332 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_UNSET_EMPTY` with `/(a)?(b)/` over `b`, replacement `[$1]` ⇒ `[]`; without it ⇒ `PCRE2_ERROR_UNSET` and `*blength` = offset into the replacement. | [ ] |
| 333 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_UNKNOWN_UNSET` alone and combined with `UNSET_EMPTY`, for all 4 sites: `$+` with no groups, `$99` (multi-digit > top_bracket, digits skipped), `$5` (single digit — reaches `length_bynumber` instead, different reported offset), `$nosuchname`. | [ ] |
| 334 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` `${name:-default}`: group set (use the group) vs unset (reprocess `text1`); nested `${a:-${b:-x}}` up to 10 levels (`PTR_STACK_SIZE` 20). | [ ] |
| 335 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` `${name:+ifset:ifunset}`: set, unset, and the empty-`text2` form `${n:+x:}`; `find_text_end` validating **both** texts eagerly. | [ ] |
| 336 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` backslash escapes: `\n \r \t \a \e \f`, `\b`⇒BS, `\v`⇒VT, `\0dd`, `\o{}`, `\xhh`, `\x{}`, `\cX`, `\N{U+41}` (UTF only), `\$`, `\\`, `\}`, `\1`..`\9`, `\10`, `\g<1>`, `\g<name>`, `\Q…\E`, lone `\E`. | [ ] |
| 337 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED\|GLOBAL` with an **unterminated** `\Q` (`\Qabc`): `escaped_literal` persists across global iterations — substitution 1 emits `abc`, later ones emit the literal `\Qabc`. | [ ] |
| 338 | `pcre2_substitute_8` | case forcing with **no** case callout (eager `default_substitute_case_callout`): `\U`, `\L`, `\u`, `\l`, `\E`, `\u\L`, `\l\U`, and `\l\U` at the very end of the replacement (not collapsed). Non-UTF table path vs `PCRE2_UTF`/`PCRE2_UCP` UCD path (incl. the 8-bit UCP-without-UTF truncation of U+0178 to `0x78`). | [ ] |
| 339 | `pcre2_substitute_8`, `pcre2_set_substitute_case_callout_8` | with a case callout (deferred `DELAYEDFORCECASE` + in-place `do_case_copy`): fast path (`\U`/`\L`/`\u\L` with `single_char == FALSE`) and split path (`\u`, `\l`, `\l\U` ⇒ `REVERSE_TITLE_FIRST`); assert `to_case` values 1/2/3 only, overlapping input/output, and the size-discovery retry loop. | [ ] |
| 340 | `pcre2_substitute_8`, `pcre2_set_substitute_case_callout_8` | case callout returning `PCRE2_SIZE_MAX` ⇒ `PCRE2_ERROR_REPLACECASE`; and a callout that inflates by more than `(len>>3)+10` combined with `OVERFLOW_LENGTH` (more than two calls needed to converge). | [ ] |
| 341 | `pcre2_substitute_8`, `pcre2_set_substitute_callout_8` | callout block fields: `version 0`, `input`, `output` base, `output_offsets[0]`/`[1]` around the replacement, `ovector`, `oveccount` (= rc, or the full count when the ovector was too small), `subscount` 1-based. | [ ] |
| 342 | `pcre2_substitute_8`, `pcre2_set_substitute_callout_8` | callout returning 0 (accept), >0 (reject: rewind + re-copy the matched text, or emit nothing under `REPLACEMENT_ONLY`), <0 (reject **and** clear GLOBAL) — with `subs` still counting the rejected substitution. | [ ] |
| 343 | `pcre2_substitute_8`, `pcre2_set_substitute_callout_8` | callout **not** invoked during an overflowed sizing pass: `*blength = 0`, `buffer = NULL`, `OVERFLOW_LENGTH`; the pessimistic `oldlength > newlength` accounting instead. | [ ] |
| 344 | `pcre2_substitute_8` | buffer sizing matrix on a known-size result: `*blength` > required, == required, == required-1 (the trailing-NUL copy overflows), and 0 with `buffer == NULL`; each with and without `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH`. Assert `*blength` out = required-1 on success, required on overflow, `PCRE2_UNSET` on `NOROOM`. | [ ] |
| 345 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_REPLACEMENT_ONLY` suppressing all 4 verbatim copies (prefix, inter-match gap, callout-rejection re-copy, trailing tail) — with and without GLOBAL; trailing NUL still written. | [ ] |
| 346 | `pcre2_substitute_8` | `PCRE2_PARTIAL_SOFT\|PCRE2_SUBSTITUTE_REPLACEMENT_ONLY` (the only legal partial pairing) on a subject that yields a full match, and one that yields `PCRE2_ERROR_PARTIAL`; `` $` `` is allowed in partial mode. | [ ] |
| 347 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` happy paths: an `rc > 0` match_data (same code pointer, same subject pointer, same length, same start offset, same non-substitute options); `rc == PCRE2_ERROR_NOMATCH` (0 substitutions, whole subject copied); `rc == 0` (ovector too small ⇒ normalised to `ovector_count`); `MATCHED\|GLOBAL` (first match from the block, later ones from `pcre2_match`). | [ ] |
| 348 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` with `PCRE2_COPY_MATCHED_SUBJECT` in the original match: the subject **pointer** may differ, contents compared with `memcmp`; `COPY_MATCHED_SUBJECT` is stripped from the internal copy so the shared buffer is not double-freed. | [ ] |
| 349 | `pcre2_substitute_8` | `PCRE2_UTF` + `PCRE2_NO_UTF_CHECK`: replacement UTF validation skipped; and without it, a valid multi-byte replacement validated once (subject validated once, `NO_UTF_CHECK` auto-set after the first match). | [ ] |
| 350 | `pcre2_substitute_8` | external `match_data` with **no** `SUBSTITUTE_MATCHED`: at EXIT the caller's `match_data->rc` is overwritten with the substitute return value (substitution count on success). Assert this side effect. | [ ] |
| 351 | `pcre2_substitute_8` | `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK` + a pattern whose `\K` moves `ovector[0]` forward: the `ovector[1] >= ovector[0] && ovector[0] >= start_offset` progress assertion, and the "empty match immediately after a non-empty match ending at the same point" allowance. | [ ] |

### 5. substring — `pcre2_substring_*_8`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 352 | `pcre2_substring_length_bynumber_8` | `/(a)(b)?/` on `a`, `oveccount 3`, `rc 2`: n=0 (whole match), n=1 (set), n=2 (unset ⇒ `PCRE2_ERROR_UNSET`), n=3 (> top_bracket ⇒ `NOSUBSTRING`, checked **before** `UNAVAILABLE`), and with `oveccount 2` n=2 (⇒ `UNAVAILABLE`). Also `sizeptr == NULL` as a pure set-probe. | [ ] |
| 353 | `pcre2_substring_length_bynumber_8` | `match_data` from a **partial** match: n=0 succeeds with size = the partial fragment length; n>0 ⇒ `PCRE2_ERROR_PARTIAL`. | [ ] |
| 354 | `pcre2_substring_length_bynumber_8` | `match_data` from a **DFA** match: no `top_bracket` check at all — n < `oveccount` and (rc==0 or n<rc) succeeds; n >= rc ⇒ `UNSET`; n >= `oveccount` ⇒ `UNAVAILABLE`. Use `/a\|ab\|abc/` with `oveccount 4`. | [ ] |
| 355 | `pcre2_substring_length_bynumber_8` | `rc == 0` (ovector too small) from `pcre2_match_8`: groups below `oveccount` are still readable (the non-DFA arm ignores `count`). | [ ] |
| 356 | `pcre2_substring_length_bynumber_8` | `\K`-produced `left > right` ⇒ rc 0 with `*sizeptr == 0`. | [ ] |
| 357 | `pcre2_substring_copy_bynumber_8` | buffer `*sizeptr` == size+1 (exactly enough), == size (one too small ⇒ `PCRE2_ERROR_NOMEMORY` with `*sizeptr` **not** updated), > size+1, and 0; plus a zero-length substring (needs `*sizeptr >= 1`); plus a DFA match_data (no guard here). | [ ] |
| 358 | `pcre2_substring_get_bynumber_8`, `pcre2_substring_free_8` | successful get on a normal, a zero-length and a NUL-containing substring; then `pcre2_substring_free_8`; plus `pcre2_substring_free_8(NULL)`. Use a custom general context to check the allocator is `match_data`'s. | [ ] |
| 359 | `pcre2_substring_nametable_scan_8` | `name_count == 0` (⇒ `NOSUBSTRING`); unique name with `firstptr != NULL` (⇒ `entrysize`, `*firstptr == *lastptr`); k duplicates (⇒ `(*lastptr-*firstptr)/entrysize+1 == k`); unique with `firstptr == NULL` (⇒ the group number); duplicated with `firstptr == NULL` (⇒ `NOUNIQUESUBSTRING`); name that is a proper prefix of a table entry; `firstptr == NULL, lastptr != NULL`. | [ ] |
| 360 | `pcre2_substring_number_from_name_8` | unique name ⇒ number; duplicate name ⇒ `NOUNIQUESUBSTRING`; absent name and `name_count == 0` ⇒ `NOSUBSTRING`. | [ ] |
| 361 | `pcre2_substring_copy_byname_8`, `pcre2_substring_get_byname_8`, `pcre2_substring_length_byname_8` | DFA match_data ⇒ `PCRE2_ERROR_DFA_UFUNC` (checked **first**, before the name is looked up) — one row per function. | [ ] |
| 362 | `pcre2_substring_copy_byname_8`, `pcre2_substring_get_byname_8`, `pcre2_substring_length_byname_8` | DUPNAMES matrix on `(?<a>x)\|(?<a>y)\|(?<a>z)`: ≥1 set (first set one, scanning first→last); all in-ovector but unset (⇒ `UNSET`); all `n >= oveccount` (⇒ `UNAVAILABLE`); mixed out-of-range + unset (⇒ `UNSET`, `failrc` upgraded); name absent (⇒ `NOSUBSTRING`). Note `NOUNIQUESUBSTRING` is never produced by these. | [ ] |
| 363 | `pcre2_substring_copy_byname_8` | unique name, group set, buffer exactly-enough and one-too-small; and a partial-match match_data where the delegate returns `PCRE2_ERROR_PARTIAL` for n>0. | [ ] |
| 364 | `pcre2_substring_list_get_8`, `pcre2_substring_list_free_8` | `rc > 0` with 1 pair and with 4 pairs, `lengthsptr != NULL` and `== NULL`; trailing/interior **unset** group (size 0, pointer to `""`, `memcpy` skipped so `subject + PCRE2_UNSET` is never formed); `\K`-produced `left > right`; `rc == 0` (⇒ `count = oveccount`, all pairs emitted); DFA match_data (no guard); `rc < 0` incl. `PCRE2_ERROR_PARTIAL` ⇒ returned verbatim, nothing allocated. Then `pcre2_substring_list_free_8` and `list_free(NULL)`. | [ ] |

### 6. pattern_info — `pcre2_pattern_info_8`, `pcre2_callout_enumerate_8`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 365 | `pcre2_pattern_info_8` | `where == NULL` length query for every request code: the 22 `uint32_t` items, `FIRSTBITMAP` (`sizeof(const uint8_t *)`), `JITSIZE`/`SIZE`/`FRAMESIZE` (`sizeof(size_t)`), `NAMETABLE` (`sizeof(PCRE2_SPTR)`) — and an unrecognized code with `where == NULL` falling through to the value switch. | [ ] |
| 366 | `pcre2_pattern_info_8` | all 27 request codes against a rich pattern `(?<a>a)(?<a>b)?(?C1)\r\n\1` compiled with `PCRE2_DUPNAMES\|PCRE2_UTF\|PCRE2_CASELESS`, `(*LIMIT_MATCH=99)`, `pcre2_set_newline(ANYCRLF)`, `pcre2_set_bsr(ANYCRLF)`: ALLOPTIONS vs ARGOPTIONS vs EXTRAOPTIONS, BACKREFMAX, CAPTURECOUNT, BSR, NEWLINE, NAMECOUNT/NAMEENTRYSIZE/NAMETABLE, HASCRORLF, JCHANGED, HASBACKSLASHC, MATCHEMPTY, MAXLOOKBEHIND, MINLENGTH, SIZE, FRAMESIZE. | [ ] |
| 367 | `pcre2_pattern_info_8` | `FIRSTCODETYPE` = 1 / 2 / 0 and `FIRSTCODEUNIT` accordingly: `abc` / `(?m)^a` / `\da`; plus `NO_START_OPTIMIZE` forcing 0. | [ ] |
| 368 | `pcre2_pattern_info_8` | `LASTCODETYPE`/`LASTCODEUNIT` = 1 (`abc`, `a.*b`) vs 0 (`a`, `abc`+ANCHORED, `a*`, `(*ACCEPT)ab`, `[Ww]ord` after the study collapse). | [ ] |
| 369 | `pcre2_pattern_info_8` | `FIRSTBITMAP` non-NULL (`[abc]x`) with the full 32 bytes asserted, vs NULL (`abc`, `(?m)^a`, `.a`). | [ ] |
| 370 | `pcre2_pattern_info_8` | `MATCHLIMIT`/`DEPTHLIMIT`/`HEAPLIMIT`: set via `(*LIMIT_*=n)` ⇒ 0 return with the value; absent ⇒ `PCRE2_ERROR_UNSET` **with the value still written** as `UINT32_MAX`. | [ ] |
| 371 | `pcre2_pattern_info_8` | `JITSIZE` with no JIT built ⇒ always 0; `FRAMESIZE` = `offsetof(heapframe, ovector) + top_bracket*2*sizeof(PCRE2_SIZE)` for top_bracket 0 and 200. | [ ] |
| 372 | `pcre2_callout_enumerate_8` | pattern exercising every opcode-skip arm of the enumerator: `(?C1)a(?C"s")[\x{100}]\p{L}*(*MARK:m)\x{100}{2,3}(?[a&&b])` under `PCRE2_UTF` — assert the full callback log (`pattern_position`, `next_item_length`, `callout_number`, string offset/length/pointer) and the `rc != 0` early-return. Also a pattern with no callouts (⇒ 0 callbacks, rc 0) and `PCRE2_AUTO_CALLOUT`. | [ ] |

### 7. serialize — `pcre2_serialize_*_8`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 373 | `pcre2_serialize_encode_8`, `pcre2_serialize_decode_8`, `pcre2_serialize_free_8`, `pcre2_code_free_8` | **1 code** round trip with `gcontext == NULL`: encode returns 1, `serialized_size = sizeof(pcre2_serialized_data) + TABLES_LENGTH + blocksize`; decode returns 1; match with the decoded code; assert the memctl/tables/executable_jit fields were zeroed in the stream (deterministic bytes for the same pattern). | [ ] |
| 374 | `pcre2_serialize_encode_8`, `pcre2_serialize_decode_8` | **many codes** (5 patterns, all default tables) round trip; `number_of_codes` on decode smaller than, equal to, and larger than `data->number_of_codes` (clamped). | [ ] |
| 375 | `pcre2_serialize_encode_8` | all codes compiled with the **same** `pcre2_maketables_8` block ⇒ accepted (the tables are serialized once). | [ ] |
| 376 | `pcre2_serialize_encode_8`, `pcre2_serialize_decode_8` | custom `pcre2_general_context_create_8` allocator on encode and a *different* one on decode: the decoded codes carry the decode-time allocator, the tables get `PCRE2_DEREF_TABLES`, and `pcre2_serialize_free_8` uses the hidden encode-time memctl. | [ ] |
| 377 | `pcre2_serialize_get_number_of_codes_8` | on a valid stream of 1 and of 5 codes. | [ ] |
| 378 | `pcre2_serialize_decode_8`, `pcre2_code_copy_8`, `pcre2_pattern_info_8` | decoded code fully usable: `pattern_info` (all items equal to the original), `match`, `substitute`, `code_copy`, `callout_enumerate`; then free all codes and the shared table block exactly once. | [ ] |
| 379 | `pcre2_serialize_free_8` | `pcre2_serialize_free_8(NULL)` — legal no-op. | [ ] |

### 8. convert — `pcre2_pattern_convert_8`, `pcre2_converted_pattern_free_8`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 380 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB` alone, `*` `?` `a` `**` patterns, two-pass (`buffptr != NULL, *buffptr == NULL`) with default separator `/` and escape `\`: expect `(?s)\A[^/]*+\z`, `(?s)\A[^/]\z`, `(?s)\Aa\z`, `(?s)\A[^/]*+\z`-style outputs; then `pcre2_converted_pattern_free_8`. | [ ] |
| 381 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB` `**` forms: `a/**` (no `\z`), `**abc` (nothing emitted for `**`), `**/abc` (`(?:\A\|/)`), `a/**/b` (`(*COMMIT)(?:.*?/)??`), `a/**x` (`(*COMMIT).*?`). | [ ] |
| 382 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR` (0x30) on `*`, `*a`, `a*`, `?`, `**`; and `PCRE2_CONVERT_GLOB_NO_STARSTAR` (0x50) on `**`; and both (0x70). | [ ] |
| 383 | `pcre2_pattern_convert_8` | glob classes: `[abc]`, `[!abc]`, `[^abc]`, `[]]`, `[!]]`, `[a-z]`, `[[:alpha:]]`, `[[:foo:]]` (unrecognized ⇒ literal), `[a-[:alpha:]]`, `[/]` and `[a-z]` spanning the separator (⇒ trailing `(?<!/)`), `[\]]` with escape. | [ ] |
| 384 | `pcre2_pattern_convert_8` | glob negated class **with** `GLOB_NO_WILD_SEPARATOR`: reproduce the stale-`out_str[2]` byte for prefixes `[!a]`, `*[!a]`, `?[!a]`, `a[!a]` exactly as the C emits them. | [ ] |
| 385 | `pcre2_pattern_convert_8`, `pcre2_set_glob_separator_8` | separator `/` (`with_escape` FALSE), `\` (TRUE), `.` (TRUE) × patterns `?`, `*`, `[!a]`, `a/**/b` — 9 distinct outputs. | [ ] |
| 386 | `pcre2_pattern_convert_8`, `pcre2_set_glob_escape_8` | escape `\` (default), `0` (no escaping), `` ` `` × patterns `a\*b`, `a\\b`, and a trailing lone escape (⇒ `PCRE2_ERROR_CONVERT_SYNTAX` with `\`, plain literal with 0). | [ ] |
| 387 | `pcre2_pattern_convert_8` | the escape+separator skip at `**`: `**\/x` and `/**\/x` with escape `\` and separator `/`. | [ ] |
| 388 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_POSIX_BASIC`: `\(a\)`, `\{2,3\}`, `\1`, `a*`, `*a` (leading `*` literal), `^a`, `a^b`, `a$`, `**`, `\.`, `[]]`, `[^]]`, `[[:alpha:]]`, `[a\]b]` — full BR translation table. | [ ] |
| 389 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_POSIX_EXTENDED`: same inputs as row 388 — assert the differences (`\(`⇒`\(`, `\1`⇒`1`, `(` counted, `)` with `bracount == 0`, `*` always a metachar, `^` always `^`). | [ ] |
| 390 | `pcre2_pattern_convert_8` | option-legality corners that are **valid**: `POSIX_BASIC \| 0x20`, `POSIX_BASIC \| 0x40`, `POSIX_EXTENDED \| 0x60` (glob-mod bits silently ignored); `GLOB \| NO_UTF_CHECK` without UTF (bit unused). | [ ] |
| 391 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_UTF` with a valid multi-byte pattern, for each of BASIC / EXTENDED / GLOB: POSIX output must be byte-identical to the non-UTF run; glob `[à-é]` becomes code-point based (vs byte-based without UTF). Also `CONVERT_UTF\|NO_UTF_CHECK`. | [ ] |
| 392 | `pcre2_pattern_convert_8` | the three buffer protocols for one glob and one POSIX pattern: `buffptr == NULL` (length only), `buffptr != NULL && *buffptr == NULL` (2-pass + malloc + `pcre2_converted_pattern_free_8`), caller-supplied buffer with `*bufflenptr` exactly the required capacity. | [ ] |
| 393 | `pcre2_pattern_convert_8` | input shapes: `pattern == NULL, plength == 0` (⇒ `(*NUL)` / `(?s)\A\z`), `plength == 0`, `plength == PCRE2_ZERO_TERMINATED`, embedded NUL with explicit length (emitted as `\`+NUL by both converters), byte `0xFF` (emitted raw). | [ ] |
| 394 | `pcre2_converted_pattern_free_8` | `pcre2_converted_pattern_free_8(NULL)` — legal no-op. | [ ] |

### 9. context — create / copy / free / setters, `pcre2_config_8`, `pcre2_maketables_8`, `pcre2_get_error_message_8`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 395 | `pcre2_general_context_create_8`, `pcre2_general_context_copy_8`, `pcre2_general_context_free_8` | `(NULL, NULL, NULL)` ⇒ defaults installed; `(mymalloc, myfree, mydata)` ⇒ counted allocations; `(mymalloc, NULL, d)` and `(NULL, myfree, d)` (one default filled in); copy then free both; `free(NULL)`. | [ ] |
| 396 | `pcre2_compile_context_create_8`, `pcre2_compile_context_copy_8`, `pcre2_compile_context_free_8`, `pcre2_set_compile_extra_options_8` | `create(NULL)` ⇒ exactly `_pcre2_default_compile_context_8` (tables = `_pcre2_default_tables_8`, max lengths `PCRE2_UNSET`, bsr `BSR_DEFAULT`, newline 2, parens 250, extra 0, varlookbehind 255, optimization_flags 7); `create(gcontext)` ⇒ memctl overridden; `pcre2_set_compile_extra_options_8` with 0, with a single bit, with the full `PUBLIC_COMPILE_EXTRA_OPTIONS` mask, and re-set to 0 (it overwrites, never ORs); copy after mutating every field; `free(NULL)`. | [ ] |
| 397 | `pcre2_match_context_create_8`, `pcre2_match_context_copy_8`, `pcre2_match_context_free_8` | `create(NULL)` ⇒ `_pcre2_default_match_context_8` (all callouts NULL, `offset_limit = PCRE2_UNSET`, heap 20000000, match 10000000, depth 10000000); `create(gcontext)`; copy after setting every callout and limit; `free(NULL)`. | [ ] |
| 398 | `pcre2_convert_context_create_8`, `pcre2_convert_context_copy_8`, `pcre2_convert_context_free_8` | `create(NULL)` ⇒ exactly `_pcre2_default_convert_context_8` (non-Windows: separator `/`, escape `\`); `create(gcontext)` ⇒ memctl overridden; copy after `set_glob_separator`/`set_glob_escape`; `free(NULL)`. Also confirm `pcre2_pattern_convert_8(…, ccontext = NULL)` uses `_pcre2_default_convert_context_8` directly. | [ ] |
| 399 | `pcre2_set_newline_8` | all 6 accepted values 1..6, each followed by a compile+match that observes the change. | [ ] |
| 400 | `pcre2_set_bsr_8` | both accepted values (`PCRE2_BSR_UNICODE`, `PCRE2_BSR_ANYCRLF`), each observed via `PCRE2_INFO_BSR` and a `\R` match. | [ ] |
| 401 | `pcre2_set_optimize_8` | every legal directive: `PCRE2_OPTIMIZATION_NONE`, `PCRE2_OPTIMIZATION_FULL`, and 64..69 (`AUTO_POSSESS`, `AUTO_POSSESS_OFF`, `DOTSTAR_ANCHOR`, `DOTSTAR_ANCHOR_OFF`, `START_OPTIMIZE`, `START_OPTIMIZE_OFF`) — assert the resulting `optimization_flags` bit pattern via a probe compile. | [ ] |
| 402 | `pcre2_set_glob_separator_8` | all 3 accepted values `/` `\` `.`; `pcre2_set_glob_escape_8` with 0 and with each of the 32 accepted punctuation bytes (assert acceptance and the resulting conversion for a representative subset). | [ ] |
| 403 | `pcre2_set_character_tables_8`, `pcre2_maketables_8`, `pcre2_maketables_free_8` | `pcre2_maketables_8(NULL)` (plain `malloc`) and with a custom gcontext; assert `TABLES_LENGTH` bytes, the lowercase / case-flip / 12 cbit class tables / ctype table content in the current locale; install via `set_character_tables`, compile, match, then `maketables_free` (both gcontext and NULL arms). | [ ] |
| 404 | `pcre2_config_8` | `where == NULL` length query for the 14 `uint32_t` items and for `JITTARGET` / `UNICODE_VERSION` / `VERSION`. | [ ] |
| 405 | `pcre2_config_8` | value query for every code: `BSR` (=UNICODE), `COMPILED_WIDTHS` (=1), `DEPTHLIMIT` (10000000), `EFFECTIVE_LINKSIZE` (=2), `HEAPLIMIT` (20000000), `JIT` (=0), `LINKSIZE` (=2), `MATCHLIMIT`, `NEWLINE` (=2), `NEVER_BACKSLASH_C` (=0), `PARENSLIMIT` (250), `STACKRECURSE` (=0), `TABLES_LENGTH`, `UNICODE` (=1); string items `UNICODE_VERSION` and `VERSION` (returned length includes the NUL). | [ ] |
| 406 | `pcre2_get_error_message_8` | every negative code -1..-76 and every compile code 100..220, with `size` large enough; plus `size` exactly `len+1` (fits), `size == len` (truncated ⇒ `PCRE2_ERROR_NOMEMORY` with the buffer still NUL-terminated), and `size == 1` (empty string + NOMEMORY). | [ ] |

### 10. low-level exported helpers — string / UTF / newline / class / study / escape

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 407 | `_pcre2_strlen_8` | length 0 (`""`), 1, many; a string with a high byte 0xFF; a 4 KiB string. | [ ] |
| 408 | `_pcre2_strcmp_8` | equal strings; first shorter (prefix); second shorter; differ at byte 0 / mid / last; both empty; differing only in a byte ≥ 0x80 (assert the `((c1 > c2) << 1) - 1` unsigned comparison, i.e. `0xFF > 0x01`). | [ ] |
| 409 | `_pcre2_strcmp_c8_8` | same matrix as row 408 against a `const char *` literal, including a `char`-signedness-sensitive high byte. | [ ] |
| 410 | `_pcre2_strncmp_8` | `len` 0 (⇒ always 0, even for different strings), 1, exactly the differing index, past a NUL in one string (NULs are compared as data, not terminators). | [ ] |
| 411 | `_pcre2_strncmp_c8_8` | same matrix as row 410 against a `const char *`. | [ ] |
| 412 | `_pcre2_strcpy_c8_8` | source `""` (⇒ returns 0, writes one NUL), 1 char, many chars, a source with a high byte; assert the return value excludes the NUL. | [ ] |
| 413 | `_pcre2_valid_utf_8` | valid inputs: length 0; pure ASCII; one 2-byte, one 3-byte and one 4-byte sequence; the maxima U+007F / U+07FF / U+FFFF / U+10FFFF; a mix; a string containing an embedded NUL. Return 0, `*erroroffset` untouched. | [ ] |
| 414 | `_pcre2_valid_utf_8` | each `PCRE2_ERROR_UTF8_ERR1..21` shape with its exact `*erroroffset` (truncated 2/3/4/5/6-byte sequences; bad 2nd–6th continuation byte; 5- and 6-byte forms; >U+10FFFF; surrogate D800–DFFF; overlong 2/3/4/5/6-byte; isolated 0x80; bytes 0xFE/0xFF). | [ ] |
| 415 | `_pcre2_ord2utf_8` | one call per `utf8_table1` band: 0x00, 0x7F (1 byte), 0x80, 0x7FF (2 bytes), 0x800, 0xFFFF (3), 0x10000, 0x10FFFF (4), and the 5- and 6-byte bands 0x200000 and 0x4000000 that the table still supports. Assert the returned count and every byte. | [ ] |
| 416 | `_pcre2_is_newline_8` | `NLTYPE_ANYCRLF` × {LF, CR alone, CR+LF, other} and `NLTYPE_ANY` × {LF, VT, FF, CR alone, CR+LF, NEL(0x85), U+2028, U+2029, other}, each with `utf == TRUE` and `utf == FALSE`; plus `ptr == endptr - 1` so the CR lookahead is out of range. Assert `*lenptr` (1 / 2 / 3, NEL is 2 in UTF and 1 otherwise). | [ ] |
| 417 | `_pcre2_was_newline_8` | mirror matrix of row 416 looking backwards: LF preceded by CR (len 2) vs not (len 1); CR, VT, FF (len 1); NEL / U+2028 / U+2029; `ptr == startptr + 1` so the CR lookbehind is out of range; `utf` TRUE (with `BACKCHAR`) and FALSE. | [ ] |
| 418 | `_pcre2_extuni_8` | `eptr == end_subject` (returns unchanged, `xcount` untouched); `\r\n` (join) and `\n\r` (break); Other→Extend / SpacingMark / ZWJ; Prepend→letter; Hangul `L+V`, `L+LV`, `LV+T`, `LVT+T` (join) and `V+L`, `T+V` (break); Control (always break). Both `xcount == NULL` and non-NULL. | [ ] |
| 419 | `_pcre2_extuni_8` | Extended_Pictographic + ZWJ + Extended_Pictographic (needs `was_ep_ZWJ`); `a` + ZWJ + Extended_Pictographic (breaks); EP + Extend + ZWJ + EP (the `lgb` is *not* updated after Extend rule); `utf` TRUE and FALSE. | [ ] |
| 420 | `_pcre2_extuni_8` | Regional indicators: 2 RIs (one cluster), 3 RIs (2+1 — odd `ricount` breaks), and a call whose `start_subject` is placed *inside* a preceding flag pair so the backwards RI count changes; `utf` TRUE (BACKCHAR walk) and FALSE. | [ ] |
| 421 | `_pcre2_script_run_8` | `ptr == endptr` (TRUE); single char incl. a `ucp_Unknown` char (TRUE); two `ucp_Unknown` (FALSE); `ab` (TRUE); Latin+Cyrillic (FALSE); Latin + combining mark (Inherited ⇒ TRUE); Common punctuation between two scripts. `utf` TRUE and FALSE. | [ ] |
| 422 | `_pcre2_script_run_8` | Han state machine: Han+Han; Han+Hiragana (⇒ HANHIRAKATA); Han+Bopomofo (⇒ HANBOPOMOFO); Han+Hangul (⇒ HANHANGUL); Hiragana+Bopomofo (FALSE); Bopomofo then Hangul (FALSE); Han followed by a char whose scriptx covers several specials (stays HANPENDING); a SCRIPT_MAP state that later switches into a HAN* state. | [ ] |
| 423 | `_pcre2_script_run_8` | digit-set consistency: `12` (ASCII fast path, TRUE); `١٢` (same Arabic-Indic set, TRUE); `1` + Arabic-Indic `٢` (FALSE); two math digits from *different* `ucd_digit_sets` entries but the same (Common) script (FALSE). | [ ] |
| 424 | `_pcre2_xclass_8` | flag combinations against a compiled `OP_XCLASS` payload: `XCL_MAP` only with `c < 256` (raw bitmap bit returned, negation already baked in) and `c >= 256`; `XCL_NOT` with and without `XCL_MAP`; `XCL_PROP`/`XCL_NOTPROP` items; the legacy sorted `XCL_SINGLE`/`XCL_RANGE`/`XCL_END` list (early exit on `c <= x`); and the packed char-list form for `c < 0x8000`, `0x8000 <= c < 0x10000`, `c >= 0x10000`, plus an empty list / `c` below the first entry (`XCL_BEGIN_WITH_RANGE`). | [ ] |
| 425 | `_pcre2_xclass_8` | one call per `PT_*` arm: LAMP, GC, PC, SC, SCX, ALNUM, SPACE, PXSPACE, WORD (note `Mn`/`Pc`, e.g. U+0300 and U+203F), CLIST, UCNC (`c < 0xa0` set `$ @ \``, and the surrogate carve-out), BIDICL, BOOL, PXGRAPH, PXPRINT, PXPUNCT, PXXDIGIT (incl. the fullwidth ranges U+FF10.. ). Note the 8-bit build forces `utf = TRUE` internally, so the `utf` argument is inert. | [ ] |
| 426 | `_pcre2_eclass_8` | `ECL_MAP` present with `c < 256` (bitmap short-circuit, expression never evaluated) vs `c >= 256`; and each RPN operator against a compiled `OP_ECLASS` body: `ECL_XCLASS` leaf, `ECL_NOT`, `ECL_AND`, `ECL_OR`, `ECL_XOR`; a 3-operand expression; and a nesting depth near the 32-operand bit-stack width. | [ ] |
| 427 | `_pcre2_update_classbits_8` | one call per `PT_*` arm × `negated` FALSE/TRUE, into a zeroed 32-byte map: assert the resulting bits for codes 0..255 and the `PT_ANY` special cases (`!negated` ⇒ `memset 0xff`, `negated` ⇒ unchanged). Cross-check the three documented divergences from `_pcre2_xclass_8` (PT_UCNC, PT_PXGRAPH/PXPRINT, PT_PXXDIGIT). | [ ] |
| 428 | `_pcre2_find_bracket_8` | over a compiled pattern containing every skip family: `number` 1..n found; `number` not present (⇒ NULL); `number < 0` finding `OP_REVERSE` and finding `OP_VREVERSE`; a pattern with `OP_XCLASS`, `OP_ECLASS`, `OP_CALLOUT_STR`, `OP_TYPE*` with `OP_PROP`/`OP_NOTPROP` (both the `+2` and the `+IMM2_SIZE+2` arms), `OP_MARK`/`OP_COMMIT_ARG`/`OP_PRUNE_ARG`/`OP_SKIP_ARG`/`OP_THEN_ARG`, and `OP_CBRA`/`OP_SCBRA`/`OP_CBRAPOS`/`OP_SCBRAPOS`; each with `utf` TRUE (multi-byte `HAS_EXTRALEN` skip) and FALSE. | [ ] |
| 429 | `_pcre2_study_8` | called directly on a freshly compiled `pcre2_real_code` (as `pcre2_compile_8` does): the `(PCRE2_FIRSTSET\|PCRE2_STARTLINE) == 0` gate; each `set_start_bits` return value (`SSB_DONE`, `SSB_CONTINUE`, `SSB_FAIL`, `SSB_TOODEEP` via 1001-deep nesting); the 1-bit and 2-bit collapse; the `UCD_CASESET != 0` veto; the `c > 127` UTF veto; the `PCRE2_LASTSET` clearing side effect. | [ ] |
| 430 | `_pcre2_study_8` | `find_minlength` gates and drivers: `PCRE2_MATCH_EMPTY` set ⇒ skipped; `PCRE2_HASACCEPT` set ⇒ skipped; `top_backref == 128` (runs) vs `129` (skipped); the `-1` returns (`\C` in UTF, `*countptr > 1000`); the `UINT16_MAX` clamp; the `MAX_CACHE_BACKREF` cache invalidation with >128 distinct backrefs. | [ ] |
| 431 | `_pcre2_auto_possessify_8` | called directly on a compiled pattern: each rewrite (`OP_STAR`/`MINSTAR`⇒`POSSTAR`, `PLUS`⇒`POSPLUS`, `QUERY`⇒`POSQUERY`, `UPTO`⇒`POSUPTO`, and the `I`/`NOT`/`TYPE` families; `OP_CR*`⇒`OP_CRPOS*` for CLASS/NCLASS/XCLASS/ECLASS); already-possessive input left alone; `rec_limit` 1000 exhaustion leaving the code unchanged. | [ ] |
| 432 | `_pcre2_auto_possessify_8` | `compare_opcodes` right-hand arms: `OP_END` (greedy ⇒ possessify, lazy ⇒ never); `OP_KET` per opening bracket (`OP_CBRA` with `cb->had_recurse`, `OP_SCRIPT_RUN`, `OP_ASSERT`, `OP_ONCE`, `OP_ASSERTBACK` with a variable-length branch, `OP_ASSERT_NA`); `OP_BRAZERO`/`OP_BRAMINZERO`; callout skipping; `OP_ALT` jump. | [ ] |
| 433 | `_pcre2_auto_possessify_8` | the three comparison arms: char-list vs each right-hand opcode (incl. the `chr < 255` vs `chr < 256` asymmetry at `OP_WORDCHAR`, tested at `chr == 255`); bitset-vs-bitset (and the 8-bit `!utf` `OP_NCLASS` inclusion); `autoposstab` (all 17×21 entries via `\D \d \S \s \W \w . \C \R \H \h \V \v \X` × `… \Z \z $ $M`) and `propposstab`/`catposstab`/`posspropstab` for `\p`/`\P` pairs. | [ ] |
| 434 | `_pcre2_auto_possessify_8` | option effects: `PCRE2_UTF` (code-point `list[2]`, `OP_NCLASS` excluded from the bitset arm, `xclass`/`eclass` calls get `utf`); `PCRE2_UCP` without UTF (`UCD_OTHERCASE` for 128–255); `PCRE2_CASELESS` (2-element char list ⇒ fewer possessifications); `PCRE2_EXTRA_CASELESS_RESTRICT` acting indirectly via the compiler (`(?i)k+\x{212a}` vs `(?i:(?r)k)+\x{212a}`). | [ ] |
| 435 | `_pcre2_check_escape_8` | `cb != NULL` (compile-time), `isclass == FALSE`: one call per return class — literal punctuation, table literals `\a \e \f \n \r \t`, each simple `ESC_*` (A B b C D d E G H h K N P p Q R S s V v W w X Z z), `\N{U+41}` under UTF, `\N{2,3}` probe, `\g<n>`/`\g{n}`/`\g1`/`\k`, `\1`..`\9`/`\10`, `\0`/`\00`/`\000`, `\o{}`, `\x{}`/`\xdd`/`\xd`, `\cX`. Assert the returned `int`, `*chptr` and the advanced `*ptrptr`. | [ ] |
| 436 | `_pcre2_check_escape_8` | `isclass == TRUE`: `\g` ⇒ literal `g`; digits are **always** octal (never a backref); `\b` ⇒ `ESC_b`; `\u{`-bad ⇒ literal `u` (not `ESC_ub`). Same inputs as row 435 for comparison. | [ ] |
| 437 | `_pcre2_check_escape_8` | `cb == NULL` (the substitute filter): only digits, `c`, `o`, `x`, `g` accepted (everything else ⇒ ERR3); `alt_bsux` forced FALSE so `\x` uses Perl syntax; `\g<n>` ⇒ `-(n+1)`, `\g<name>` ⇒ `ESC_g`. Drive with `bracount` 0 and 5. | [ ] |
| 438 | `_pcre2_check_escape_8` | option-driven results: `PCRE2_ALT_BSUX` and `PCRE2_EXTRA_ALT_BSUX` (`\u`, `\u{}`, `\x`, `\U`); `PCRE2_EXTRA_PYTHON_OCTAL` vs default for `\12`/`\123`/`\8`; `PCRE2_EXTRA_NO_BS0`; `PCRE2_EXTRA_ESCAPED_CR_IS_LF`; `PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES` for `\u`/`\o{}`/`\x{}`; `PCRE2_UTF` limits (0x10FFFF vs 0xFF); `PCRE2_UCP`; `bracount` driving the digit/backref split. | [ ] |
| 439 | `_pcre2_ckd_smul_8` | `(0,0)`, `(1,1)`, `(65535,65535)`, `(INT_MAX,1)`, `(65536,65536)`, `(46341,46341)` and a pair that overflows `PCRE2_SIZE` on this LP64 build — assert both the BOOL return and `*r` when it returns FALSE. | [ ] |
| 440 | `_pcre2_memctl_malloc_8` | `memctl == NULL` (⇒ plain `malloc`, default `malloc`/`free` written into the header) and `memctl != NULL` (⇒ the supplied allocator, header copied); size exactly `sizeof(pcre2_memctl)` and larger; verify the returned block's embedded memctl so the matching free works. | [ ] |
| 441 | `_pcre2_compile_get_hash_from_name8` | length 1 (`a`), length 2 (`ab`), length 128; names differing only in the first byte, only in the last byte; a high-byte name (⇒ `(name[0] & 0x7f) \| ((name[len-1] & 0xff) << 7)` and the `<= NAMED_GROUP_HASH_MASK` invariant). | [ ] |
| 442 | `_pcre2_compile_find_named_group8`, `_pcre2_compile_add_name_to_table8`, `_pcre2_compile_find_dupname_details8` | exercised through compiles that hit each branch: found / not found; hash collision with a different name; in-order and out-of-order table insertion (`memmove`); the substring-name `crc = -1` fix-up; `NAMED_GROUP_IS_DUPNAME` with `duplicate_count` 1, 2 and 3; dupname index/count lookup used by `\k<n>` and `(?(<n>))`. | [ ] |
| 443 | `_pcre2_compile_class_not_nested_8`, `_pcre2_compile_class_nested_8` | exercised through compiles: `not_nested` producing each of `OP_CLASS`, `OP_NCLASS`, `OP_XCLASS`, `OP_ALLANY` (with `has_bitmap` set and clear); `nested` on a `META_CLASS`/`META_CLASS_NOT` pair from `(?[…])` and from `PCRE2_ALT_EXTENDED_CLASS`, both in the length-computing pass (`lengthptr != NULL`) and the emitting pass. | [ ] |
| 444 | `_pcre2_compile_parse_scan_substr_args8`, `_pcre2_compile_parse_recurse_args8` | exercised through compiles: `(*scs:1)`, `(*scs:1,2)`, `(*scs:<a>)`, `(*scs:<a>,<b>)` with DUPNAMES; `(?1(2))`, `(?1(2,3))`, `(?&x(<y>,<z>))` including a duplicate argument (dedup) and out-of-order arguments (heapsort); both compile passes. | [ ] |

### 11. tables — exported read-only data

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 445 | `_pcre2_OP_lengths_8` | full byte-for-byte dump compared against the C symbol; plus a consistency check that walking every opcode emitted by a rich pattern with these lengths lands exactly on `OP_END`. | [ ] |
| 446 | `_pcre2_default_tables_8` | full `TABLES_LENGTH`-byte dump: lowercase table, case-flip table, the 12 cbit class bitmaps (`cbit_digit/upper/lower/word/space/xdigit/graph/print/punct/cntrl`), and the 256-byte ctype table. | [ ] |
| 447 | `_pcre2_hspace_list_8`, `_pcre2_vspace_list_8` | full dumps; cross-checked against the `\h`/`\v` match behaviour in UTF and non-UTF. | [ ] |
| 448 | `_pcre2_utf8_table1`, `_pcre2_utf8_table1_size`, `_pcre2_utf8_table2`, `_pcre2_utf8_table3`, `_pcre2_utf8_table4` | full dumps; `utf8_table1_size == 6`; cross-checked against `_pcre2_ord2utf_8` band selection and `_pcre2_valid_utf_8` additional-byte counts. | [ ] |
| 449 | `_pcre2_utt_8`, `_pcre2_utt_names_8`, `_pcre2_utt_size_8` | full dump of the `ucp_type_table` array and the name blob; every entry resolvable via a `\p{name}` compile (loose matching, `sc:`/`scx:`/`bc:` prefixes). | [ ] |
| 450 | `_pcre2_ucp_gentype_8`, `_pcre2_ucp_gbtable_8`, `_pcre2_posix_class_maps8`, `_pcre2_callout_start_delims_8`, `_pcre2_callout_end_delims_8` | full dumps; `ucp_gbtable` cross-checked against `_pcre2_extuni_8` join/break decisions; `posix_class_maps` against `[[:name:]]` compiles; the delimiter arrays against the 8 accepted `(?C…)` delimiters. | [ ] |
| 451 | `_pcre2_ucd_records_8`, `_pcre2_ucd_stage1_8`, `_pcre2_ucd_stage2_8`, `_pcre2_ucd_caseless_sets_8`, `_pcre2_ucd_boolprop_sets_8`, `_pcre2_ucd_script_sets_8`, `_pcre2_ucd_digit_sets_8`, `_pcre2_ucd_nocase_ranges_8`, `_pcre2_ucd_nocase_ranges_size_8`, `_pcre2_ucd_turkish_dotted_i_caseset_8`, `_pcre2_unicode_version_8` | full dumps of all 11 symbols; plus a per-code-point sweep (0..0x10FFFF, sampled) of `UCD_CHARTYPE`/`UCD_SCRIPT`/`UCD_OTHERCASE`/`UCD_CASESET`/`UCD_GRAPHBREAK`/`UCD_BIDICLASS`/`UCD_SCRIPTX_PROP`/`UCD_BPROPS_PROP` through the two-stage tables. | [ ] |

### 12. jit stubs — no-JIT arms

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 452 | `pcre2_jit_compile_8` | `options == PCRE2_JIT_TEST_ALLOC` alone ⇒ `PCRE2_ERROR_JIT_UNSUPPORTED`; `PCRE2_JIT_TEST_ALLOC \| anything` ⇒ `PCRE2_ERROR_JIT_BADOPTION`. | [ ] |
| 453 | `pcre2_jit_compile_8` | `code == NULL` ⇒ `PCRE2_ERROR_NULL`; a bit outside `PUBLIC_JIT_COMPILE_OPTIONS` ⇒ `PCRE2_ERROR_JIT_BADOPTION`; `PCRE2_JIT_COMPLETE` / `PARTIAL_SOFT` / `PARTIAL_HARD` on a valid code ⇒ `PCRE2_ERROR_JIT_BADOPTION`. | [ ] |
| 454 | `pcre2_jit_compile_8`, `pcre2_pattern_info_8` | `PCRE2_JIT_INVALID_UTF` on a code compiled **without** `PCRE2_MATCH_INVALID_UTF`: returns `PCRE2_ERROR_JIT_BADOPTION` **but** has already OR-ed `PCRE2_MATCH_INVALID_UTF` into `re->overall_options` — assert the observable ALLOPTIONS change and that a later `pcre2_match_8` now uses the invalid-UTF fragment logic. | [ ] |
| 455 | `pcre2_jit_stack_create_8`, `pcre2_jit_stack_assign_8`, `pcre2_jit_stack_free_8`, `pcre2_jit_free_unused_memory_8` | `stack_create(1, 1024, NULL)` and `(0, 0, gcontext)` ⇒ NULL in both cases; `stack_assign(mcontext, cb, data)` ⇒ no-op (mcontext unchanged, later matches unaffected); `stack_free(NULL)` and `stack_free(ptr)`; `jit_free_unused_memory(NULL)` and with a gcontext. | [ ] |
| 456 | `_pcre2_jit_get_target_8`, `_pcre2_jit_get_size_8`, `_pcre2_jit_free_8`, `_pcre2_jit_free_rodata_8` | `jit_get_target()` ⇒ the literal `"JIT is not supported"`; `jit_get_size(NULL)` and `jit_get_size(ptr)` ⇒ 0; `jit_free(NULL, memctl)` and `jit_free(ptr, memctl)` ⇒ no-op; `jit_free_rodata(NULL, NULL)` and with non-NULL arguments ⇒ no-op. | [ ] |

