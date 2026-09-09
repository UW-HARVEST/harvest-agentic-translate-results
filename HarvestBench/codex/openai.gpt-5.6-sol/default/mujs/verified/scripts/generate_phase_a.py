#!/usr/bin/env python3
"""Mechanically regenerate the MuJS Phase A verification inventories."""

from __future__ import annotations

import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
C_ROOT = ROOT / "c_src"
CRATE = ROOT / "translation"
C_SO = C_ROOT / "build" / "libmujs.so"
RUST_SO = CRATE / "target" / "release" / "libmujs.so"


def dynamic_symbols(path: Path) -> list[str]:
    output = subprocess.check_output(
        ["nm", "-D", "--defined-only", str(path)], text=True
    )
    symbols = []
    for line in output.splitlines():
        fields = line.split()
        if len(fields) >= 3 and fields[-2] in {"T", "D", "B", "R"}:
            symbols.append(fields[-1])
    return sorted(set(symbols))


def markdown_cell(text: str) -> str:
    return " ".join(text.replace("|", r"\|").split())


def function_map(lines: list[str]) -> list[str]:
    result = ["<file-scope>"] * len(lines)
    depth = 0
    current = "<file-scope>"
    signature = ""
    controls = {"if", "for", "while", "switch"}
    for index, line in enumerate(lines):
        stripped = line.strip()
        if depth == 0:
            if signature:
                signature += " " + stripped
            elif "(" in stripped and not stripped.startswith(("#", "typedef")):
                signature = stripped
            if signature and ";" in signature and (
                "{" not in signature or signature.index(";") < signature.index("{")
            ):
                signature = ""
            if signature and "{" in signature:
                prefix = signature.split("(", 1)[0].strip()
                match = re.search(r"([A-Za-z_][A-Za-z0-9_]*)$", prefix)
                name = match.group(1) if match else "<file-scope>"
                if name in controls:
                    signature = ""
                else:
                    current = name
                    signature = ""
        result[index] = current
        depth += line.count("{") - line.count("}")
        if depth <= 0 and not signature:
            depth = 0
            current = "<file-scope>"
    return result


ERROR_CALL = re.compile(
    r"\b("
    r"js_(?:error|evalerror|rangeerror|referenceerror|syntaxerror|typeerror|urierror)"
    r"|js[PYC]_error|js_(?:outofmemory|stackoverflow|runlimit|trystackoverflow)"
    r"|die"
    r")\s*\("
)
RETURN_SENTINEL = re.compile(r"\breturn\s+(-1|NULL|NAN)\s*;")
ASSERT = re.compile(r"\bassert\s*\((.*)\)\s*;")


def nearest_condition(lines: list[str], line_index: int) -> str:
    line = lines[line_index].strip()
    inline = re.search(r"\bif\s*\((.*?)\)", line)
    if inline:
        return inline.group(1)
    for offset in range(1, 5):
        if line_index - offset < 0:
            break
        candidate = lines[line_index - offset].strip()
        match = re.search(r"\b(?:if|else if)\s*\((.*)\)", candidate)
        if match:
            return match.group(1)
        if candidate.endswith((";", "}")) and not candidate.startswith("if"):
            break
    return f"control reaches this rejection site ({line.strip()})"


def expected_result(line: str) -> str:
    match = RETURN_SENTINEL.search(line)
    if match:
        return f"returns `{match.group(1)}`"
    match = ASSERT.search(line)
    if match:
        return f"`assert({markdown_cell(match.group(1))})` must hold; otherwise abort"
    match = ERROR_CALL.search(line)
    assert match
    name = match.group(1)
    kind = {
        "js_error": "Error",
        "js_evalerror": "EvalError",
        "js_rangeerror": "RangeError",
        "js_referenceerror": "ReferenceError",
        "js_syntaxerror": "SyntaxError",
        "js_typeerror": "TypeError",
        "js_urierror": "URIError",
        "jsP_error": "SyntaxError",
        "jsY_error": "SyntaxError",
        "jsC_error": "SyntaxError",
        "js_outofmemory": "out-of-memory error",
        "js_stackoverflow": "stack-overflow error",
        "js_runlimit": "run-limit error",
        "js_trystackoverflow": "exception-stack-overflow error",
        "die": "regexp compile failure (`NULL` plus error string)",
    }[name]
    return f"raises/reports {kind}"


def error_rows() -> list[tuple[str, str, str]]:
    rows: list[tuple[str, str, str]] = []
    for path in sorted((C_ROOT / "src").glob("*.c")):
        lines = path.read_text(errors="replace").splitlines()
        functions = function_map(lines)
        for index, line in enumerate(lines):
            stripped = line.strip()
            call = ERROR_CALL.search(stripped)
            is_definition = bool(
                call
                and functions[index] == call.group(1)
                and stripped.endswith(")")
                and not stripped.startswith(("if", "return"))
            )
            if (call and not is_definition) or RETURN_SENTINEL.search(
                stripped
            ) or ASSERT.search(stripped):
                location = f"{functions[index]} (`{path.name}:{index + 1}`)"
                trigger = nearest_condition(lines, index)
                rows.append(
                    (
                        location,
                        markdown_cell(trigger),
                        markdown_cell(expected_result(stripped)),
                    )
                )
    return rows


def symbol_source(symbol: str) -> str:
    pattern = re.compile(rf"\b{re.escape(symbol)}\s*\(")
    for path in [C_ROOT / "include" / "mujs.h", *sorted((C_ROOT / "src").glob("*.[ch]"))]:
        for number, line in enumerate(path.read_text(errors="replace").splitlines(), 1):
            if pattern.search(line):
                return f"{path.relative_to(C_ROOT)}:{number}"
    return "macro/generated"


CONFIG_AXES = [
    ("js_newstate/js_freestate", "state flags: `0`, `JS_STRICT`, and unknown bits; default allocator; empty state lifecycle"),
    ("js_newstate", "allocator shapes: default allocator, custom successful allocator, allocator failure on state allocation, stack allocation, and initialization"),
    ("js_setcontext/js_getcontext", "context pointer shapes: null and non-null opaque pointers"),
    ("js_setreport/js_report", "report callback shapes: default, custom callback, empty message, and non-ASCII message"),
    ("js_atpanic", "panic callback replacement: null/default and custom callback"),
    ("js_setlimit/js_dostring", "run/memory limits: disabled, exact boundary, one step below required work, and generous"),
    ("js_gc", "GC report flag `0` and nonzero; empty, live-object, and garbage-heavy heaps"),
    ("js_dostring/js_ploadstring/js_loadstring/js_eval", "source shapes: empty, expression, statement list, function, strict directive, comments, UTF-8, and embedded line terminators"),
    ("js_dostring/js_ploadstring", "valid parser grammar families: literals, arrays, objects, accessors, functions, loops, switch, try/catch/finally, regexps, and JSON"),
    ("js_pcall/js_call", "argument counts: zero, one, and many; JS function and C function; primitive and object return"),
    ("js_pconstruct/js_construct", "constructors: JS and C constructors; zero/many arguments; primitive/object return; custom/default prototype"),
    ("js_ref/js_unref", "registry references: primitive/object values; one and many references; valid and missing reference names"),
    ("js_getregistry/js_setregistry/js_delregistry", "registry keys: empty, ASCII, UTF-8, existing, and missing"),
    ("js_getglobal/js_setglobal/js_defglobal/js_delglobal", "global keys and attributes: missing/existing; all 8 `READONLY|DONTENUM|DONTCONF` combinations"),
    ("js_hasproperty/js_getproperty/js_setproperty/js_defproperty/js_delproperty", "object property shapes: own/inherited/missing; data/accessor; extensible/non-extensible; all attribute combinations"),
    ("js_defaccessor", "accessor shapes: getter-only, setter-only, both; configurable/non-configurable"),
    ("js_getlength/js_setlength", "length shapes: empty, one, many, sparse, shrink, grow, and maximum-adjacent"),
    ("js_hasindex/js_getindex/js_setindex/js_delindex", "index shapes: negative, zero, interior, end, beyond end; dense array, sparse array, string, and ordinary object"),
    ("js_currentfunction/js_currentfunctiondata", "outside/inside C function; null/non-null function data"),
    ("js_pushundefined/js_pushnull/js_pushboolean/js_pushnumber/js_pushstring/js_pushlstring", "value shapes: every primitive; false/true; -0, finite, infinities, NaN; empty/short/heap/UTF-8/embedded-NUL strings"),
    ("js_newobjectx/js_newobject/js_newarray", "container shapes: empty; one/many properties; dense/sparse array; prototype and no-prototype"),
    ("js_newboolean/js_newnumber/js_newstring", "boxed primitives across boolean, numeric boundary, and string-size shapes"),
    ("js_newcfunction/js_newcfunctionx/js_newcconstructor", "native callable shapes: zero/many arity, null/non-null data, finalizer absent/present, function/constructor paths"),
    ("js_newuserdata/js_newuserdatax", "userdata shapes: null/non-null data; matching/mismatching tags; has/put/delete/finalize callbacks absent/present"),
    ("js_newregexp/js_regcomp/js_regcompx/js_regexec", "all 8 `G|I|M`/`ICASE|NEWLINE|NOTBOL` flag combinations; empty/literal/class/group/anchor/backreference patterns; empty/ASCII/UTF-8/multiline haystacks"),
    ("js_pushiterator/js_nextiterator/jsV_newiterator/jsV_nextiterator", "iterator modes: own-only and inherited; empty/one/many properties; enumerable/non-enumerable; array/string/object"),
    ("js_isdefined/js_isundefined/js_isnull/js_isboolean/js_isnumber/js_isstring/js_isprimitive/js_isobject", "every JS value category and valid positive/negative stack indices"),
    ("js_isarray/js_isregexp/js_iscallable/js_isuserdata/js_iserror", "matching and non-matching object subtypes"),
    ("js_isnumberobject/js_isstringobject/js_isbooleanobject/js_isdateobject", "boxed matching type, different boxed type, primitive, and ordinary object"),
    ("js_toboolean/jsV_toboolean", "truthiness categories: undefined, null, false/true, ±0, NaN, finite nonzero, empty/nonempty string, object"),
    ("js_tonumber/jsV_tonumber/jsV_stringtonumber/js_stringtofloat/js_strtod/js_strtol", "numeric text/value shapes: whitespace, sign, integer, fraction, exponent, hex, Infinity, NaN/invalid, overflow, underflow, and trailing text"),
    ("js_tostring/jsV_tostring/jsV_numbertostring/js_fmtexp/js_grisu2/js_itoa", "number/string formatting: -0, integer bounds, fractions, exponent thresholds, infinities, NaN, bases 2/10/16/36"),
    ("js_trystring/js_trynumber/js_tryinteger/js_tryboolean/js_tryrepr", "successful conversion and throwing conversion; caller-provided fallback sentinel values"),
    ("js_tointeger/js_toint32/js_touint32/js_toint16/js_touint16", "numeric conversion boundaries: NaN/±Infinity/±0, min/max, one-step outside, fractions, and modulo wrap"),
    ("js_gettop/js_pop/js_rot/js_copy/js_remove/js_insert/js_replace", "stack shapes: zero/one/many elements; positive/negative indices; first/interior/last positions"),
    ("js_dup/js_dup2/js_rot2/js_rot3/js_rot4/js_rot2pop1/js_rot3pop2", "minimum valid stack size and larger stacks with distinct marker values"),
    ("js_concat", "primitive combinations: string+string, string+number, number+number, object coercion"),
    ("js_compare/js_equal/js_strictequal", "same/different type pairs; null/undefined; booleans/numbers/strings/objects; NaN and coercion"),
    ("js_instanceof", "matching/nonmatching prototype chain; primitive LHS; callable/non-callable RHS"),
    ("js_typeof/js_type", "all seven public type enum results, including function/object distinction"),
    ("js_repr/js_torepr/js_tryrepr", "all primitive/object subtypes; cycles; sparse arrays; escaping ASCII/control/UTF-8; throwing accessors"),
    ("jsU_chartorune/jsU_runetochar/jsU_runelen/js_runeat/js_utflen/js_utfptrtoidx", "UTF-8 shapes: ASCII, 2/3/4-byte scalar, overlong NUL, malformed sequence, surrogate, max scalar, and out-of-range rune"),
    ("jsU_isalpharune/jsU_islowerrune/jsU_isupperrune/jsU_tolowerrune/jsU_toupperrune", "Unicode categories: ASCII/non-ASCII letters, lower/upper/title, uncased, and expansion mappings"),
    ("js_isarrayindex", "property-name shapes: empty, `0`, leading zero, decimal interior, `INT_MAX`, overflow, negative, and nondigit"),
    ("jsV_newmemstring/js_intern/js_strdup", "string allocation/interning: empty, inline boundary, heap boundary, duplicate, UTF-8, and maximum-adjacent"),
    ("jsV_newobject/jsV_resizearray/jsR_unflattenarray", "object/array storage transitions: empty, flat dense growth, sparse conversion, shrink, and boundary-adjacent"),
    ("jsV_getownproperty/jsV_getproperty/jsV_getpropertyx/jsV_setproperty/jsV_delproperty", "low-level property paths: own/prototype/missing; array/string special properties; data/accessor/readonly/nonconfigurable"),
    ("jsP_parse/jsP_parsefunction/jsC_compilescript/jsC_compilefunction", "low-level parse/compile: empty/simple/compound/deep source; script vs function; strict vs non-strict"),
    ("jsY_initlex/jsY_lex/jsY_lexjson/jsY_tokenstring/jsY_findword", "lexer shapes: every token family, keywords/identifiers, numeric/string/regexp escapes, JSON mode, EOF, and UTF-8"),
]


def main() -> None:
    c_symbols = dynamic_symbols(C_SO)
    rust_symbols = dynamic_symbols(RUST_SO)
    rust_set = set(rust_symbols)

    symbol_lines = [
        "# Dynamic symbol surface",
        "",
        f"Generated mechanically from `nm -D --defined-only`. C exports: **{len(c_symbols)}**; Rust exports: **{len(rust_symbols)}**.",
        "",
        "| # | C symbol | C source | Rust `.so` |",
        "|---:|----------|----------|------------|",
    ]
    for number, symbol in enumerate(c_symbols, 1):
        status = "[x] exact export" if symbol in rust_set else "[ ] MISSING"
        symbol_lines.append(
            f"| {number} | `{symbol}` | `{symbol_source(symbol)}` | {status} |"
        )
    missing = sorted(set(c_symbols) - rust_set)
    extra = sorted(rust_set - set(c_symbols))
    symbol_lines += [
        "",
        f"Missing from Rust: **{len(missing)}**. Extra Rust exports: **{len(extra)}**.",
        "",
    ]
    (CRATE / "SYMBOLS.md").write_text("\n".join(symbol_lines))

    errors = error_rows()
    error_lines = [
        "# Error surface",
        "",
        "Generated from every C source occurrence of an explicit MuJS error/throw helper, regexp `die`, `assert`, or `return -1/NULL/NAN` sentinel. The source location keeps same-message branches distinct.",
        "",
        "| # | function | trigger (the exact invalid input/condition) | expected C result |",
        "|---:|----------|---------------------------------------------|-------------------|",
    ]
    for number, (function, trigger, result) in enumerate(errors, 1):
        error_lines.append(
            f"| {number} | {function} | `{trigger}` | {result} [ ] |"
        )
    error_lines.append("")
    (CRATE / "ERRORS.md").write_text("\n".join(error_lines))

    config_lines = [
        "# Configuration surface",
        "",
        "The first section is the complete `nm -D` entry-point surface. The second is the source-derived cross-product of public flags, value categories, input shapes, and low-level API families that C branches on.",
        "",
        "## Every exported entry point",
        "",
        "| # | entry point(s) | configuration (options set + input shape) | [ ] |",
        "|---:|----------------|-------------------------------------------|-----|",
    ]
    number = 1
    for symbol in c_symbols:
        config_lines.append(
            f"| {number} | `{symbol}` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`{symbol_source(symbol)}`) | [ ] |"
        )
        number += 1
    config_lines += [
        "",
        "## Branch-distinct option and data-shape combinations",
        "",
        "| # | entry point(s) | configuration (options set + input shape) | [ ] |",
        "|---:|----------------|-------------------------------------------|-----|",
    ]
    for entries, configuration in CONFIG_AXES:
        config_lines.append(
            f"| {number} | `{entries}` | {configuration} | [ ] |"
        )
        number += 1
    config_lines.append("")
    (CRATE / "CONFIGS.md").write_text("\n".join(config_lines))


if __name__ == "__main__":
    main()
