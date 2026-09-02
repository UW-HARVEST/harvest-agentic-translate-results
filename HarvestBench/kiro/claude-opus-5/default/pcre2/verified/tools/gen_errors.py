#!/usr/bin/env python3
"""Generate the compile-error section of ERRORS.md mechanically.

Rows come from:
  * the ERRn -> code mapping in c_src/src/pcre2_compile.h (ERR0 = 100)
  * the error names in c_src/include/pcre2.h
  * the actual raising sites found by dump_errors.py
  * the trigger + covering test taken from translation/tests/errors_compile.rs
"""
import os, re, subprocess, collections

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
HDR = os.path.join(ROOT, "c_src", "include", "pcre2.h")
TESTS = os.path.join(ROOT, "translation", "tests", "errors_compile.rs")

names = {}
for line in open(HDR):
    m = re.match(r"#define\s+PCRE2_ERROR_([A-Z0-9_]+)\s+(\d+)\s*$", line)
    if m:
        names[int(m.group(2))] = m.group(1)

# raising sites
sites = collections.defaultdict(list)
out = subprocess.run(["python3", os.path.join(os.path.dirname(os.path.abspath(__file__)), "dump_errors.py"), "ERRn"],
                     capture_output=True, text=True).stdout
for line in out.splitlines():
    loc, fn, tok, _txt = line.split("\t")
    if loc.startswith("pcre2_compile.h"):
        continue
    n = int(tok[3:])
    sites[n].append(loc)

# triggers from the Rust corpus
trig = {}
for line in open(TESTS):
    m = re.match(r'\s*c\("([A-Za-z0-9_-]+)",\s*(b?r?"(?:[^"\\]|\\.)*"|b"(?:[^"\\]|\\.)*"),\s*(.*?),\s*(.*?),\s*(-?\d+)\),\s*$', line)
    if m:
        row, pat, opts, xopts, exp = m.groups()
        exp = int(exp)
        if exp:
            trig.setdefault(exp, []).append((row, pat, opts.strip(), xopts.strip()))

# manual notes for rows covered by dedicated tests or not reachable in this build
special = {
    101: ("pattern `\\` (a lone trailing backslash)", "`compile_error_corpus`"),
    116: ("compile(NULL, 5, ...)", "`err16_null_pattern_with_nonzero_length`"),
    119: ("251 nested `(` (default PARENS_NEST_LIMIT 250), or `set_parens_nest_limit(n)` then n+1 levels",
          "`err19_parentheses_nest_too_deep`, `generated_oversize_patterns`"),
    120: ("compiled size > MAX_PATTERN_SIZE (1<<16 at LINK_SIZE 2): 40000 literal code units",
          "`generated_oversize_patterns`"),
    121: ("general context whose `malloc` always returns NULL", "`err21_heap_failed_via_failing_allocator`"),
    133: ("`pcre2_set_compile_recursion_guard()` callback returns non-zero", "`err33_recursion_guard_rejects`"),
    135: ("> 2000 length-computation steps: `(?<=` + 1500 x `(?|a|b)` + `)x`", "`generated_oversize_patterns`"),
    148: ("`(?<` + 129 x `a` + `>x)` (MAX_NAME_SIZE 128)", "`err48_subpattern_name_too_long_boundary`"),
    149: ("10001 distinct named groups (MAX_NAME_COUNT 10000)", "`err97_too_many_captures_and_err49_too_many_names`"),
    176: ("`(*MARK:` + 256 x `a` + `)`", "`err76_verb_name_too_long_boundary`"),
    172: ("`(?C{` + 70000 x `x` + `})a`", "`err72_callout_string_too_long_boundary`"),
    184: ("300 nested `(?|`", "`err84_query_barjx_nest_too_deep`"),
    187: ("`(?<=a{65536})b`", "`err87_lookbehind_too_long`"),
    188: ("`set_max_pattern_length(2)` then a 5-unit pattern", "`err88_pattern_string_too_long`"),
    197: ("> 65535 capture groups", "`err97_too_many_captures_and_err49_too_many_names`"),
    200: ("`set_max_varlookbehind(1)` then `(?<=ab|cd)`", "`err100_max_varlookbehind_exceeded`"),
    201: ("`set_max_pattern_compiled_length(1)` then `(abc)+d[e-g]{2,4}`", "`err101_pattern_compiled_size_too_big`"),
    207: ("300 nested `[` inside `(?[...])`", "`err107_eclass_nest_too_deep`"),
    220: ("compile(..., erroroffset = NULL)", "`err120_null_erroroffset`"),
}

unreachable = {
    132: "`#ifndef SUPPORT_UNICODE` — this build defines SUPPORT_UNICODE, so the branch is compiled out.",
    145: "`#ifndef SUPPORT_UNICODE` — compiled out in this build.",
    185: "`#ifdef NEVER_BACKSLASH_C` — not defined in config.h, so compiled out.",
    191: "`#if PCRE2_CODE_UNIT_WIDTH == 16` — this build is 8-bit, so compiled out.",
    196: "`#ifndef SUPPORT_UNICODE` — compiled out in this build.",
    110: "internal invariant (`PCRE2_DEBUG_UNREACHABLE` / `LCOV_EXCL`): not reachable from the public API.",
    123: "internal invariant: code-block overflow, not reachable from the public API.",
    131: "internal invariant in `_pcre2_study()`.",
    152: "internal invariant: workspace overrun (guarded by `PCRE2_DEBUG_UNREACHABLE`).",
    153: "internal invariant: previously-checked group missing.",
    156: "internal invariant: newline type already validated by `pcre2_set_newline`.",
    163: "internal invariant: parsed-pattern overflow (sized from the pattern length).",
    170: "internal invariant: unrecognized meta code in `check_lookbehinds()`.",
    180: "internal invariant: unknown opcode in `_pcre2_auto_possessify()`.",
    186: "workspace safety-margin check: not reachable through the public API in this configuration (the largest single item we could build, a 6000-range XCLASS, stays inside the margin).",
    189: "internal invariant: bad code value.",
    190: "internal invariant: bad code value in `parsed_skip()`.",
    159: "code ERR59 is never assigned anywhere in the C source (\"obsolete error\").",
}

rows = []
for code in sorted(names):
    n = code - 100
    nm = names[code]
    site = ", ".join(sorted(set(sites.get(n, []))))
    if code in trig:
        ts = trig[code]
        t = "; ".join(
            f"pattern {pat}" + (f", opts `{o}`" if o != "0" else "") + (f", xopts `{x}`" if x != "0" else "")
            for _row, pat, o, x in ts
        )
        cov = "`compile_error_corpus`"
    elif code in special:
        t, cov = special[code]
    elif code in unreachable:
        t = "**not reachable in this build** — " + unreachable[code]
        cov = "n/a (see note); randomized fuzzing would surface any divergence"
    else:
        t = "(no dedicated trigger)"
        cov = "`cfg_random_patterns_fuzz`"
    rows.append((code, f"ERR{n}", nm, site, t, cov))

with open(os.path.join(ROOT, "translation", "ERRORS_compile_rows.md"), "w") as f:
    f.write("| # | code | `ERRn` | name | raised at | trigger (exact invalid input/condition) | expected C result | covering test |\n")
    f.write("|---|------|--------|------|-----------|------------------------------------------|-------------------|---------------|\n")
    for i, (code, errn, nm, site, t, cov) in enumerate(rows, 1):
        f.write(f"| C{i} | {code} | `{errn}` | `PCRE2_ERROR_{nm}` | {site or '—'} | {t} | "
                f"`pcre2_compile` returns NULL, `*errorcode == {code}`, `*erroroffset` set | {cov} |\n")
print("rows:", len(rows))
