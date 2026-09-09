#!/usr/bin/env python3
"""Mechanically generate Phase A inventories from the C build and sources."""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CRATE = ROOT / "translation"
C_ROOT = ROOT / "c_src"
C_SO = C_ROOT / "build" / "libpcre2.so"
RUST_SO = CRATE / "target" / "release" / "libpcre2.so"
HEADER = C_ROOT / "include" / "pcre2.h"


def dynamic_symbols(path: Path) -> list[str]:
    output = subprocess.check_output(
        ["nm", "-D", "--defined-only", str(path)], text=True
    )
    return sorted({line.split()[-1] for line in output.splitlines() if line.split()})


def write_symbols() -> None:
    c_symbols = dynamic_symbols(C_SO)
    rust_symbols = set(dynamic_symbols(RUST_SO))
    lines = [
        "# Dynamic symbol surface",
        "",
        f"C library: `{C_SO}`",
        "",
        f"Rust library: `{RUST_SO}`",
        "",
        "| # | C symbol | Rust export |",
        "|---:|----------|-------------|",
    ]
    for number, symbol in enumerate(c_symbols, 1):
        state = "[x] present" if symbol in rust_symbols else "[ ] MISSING"
        lines.append(f"| {number} | `{symbol}` | {state} |")
    missing = sorted(set(c_symbols) - rust_symbols)
    extra = sorted(rust_symbols - set(c_symbols))
    lines += [
        "",
        f"Missing from Rust: **{len(missing)}**",
        "",
        f"Extra in Rust: **{len(extra)}**",
    ]
    if missing:
        lines += ["", "## Missing symbols", ""] + [f"- `{s}`" for s in missing]
    if extra:
        lines += ["", "## Extra symbols", ""] + [f"- `{s}`" for s in extra]
    (CRATE / "SYMBOLS.md").write_text("\n".join(lines) + "\n")


FUNCTION_START = re.compile(
    r"^(?:PCRE2_EXP_DEFN\s+)?(?:static\s+)?(?:const\s+)?"
    r"(?:[\w*]+\s+)+(?P<name>[A-Za-z_]\w*)\s*\([^;]*$"
)
ERROR_RETURN = re.compile(
    r"\breturn\s+(?P<result>"
    r"PCRE2_ERROR_[A-Z0-9_]+|ERR\d+|NULL|-1|-2|-3|0xffffffffu|FALSE"
    r")\s*;"
)
ASSERTION = re.compile(r"\b(?:PCRE2_ASSERT|SLJIT_ASSERT|assert)\s*\((?P<expr>.*)")
CHECK_PREFIX = re.compile(r"^\s*(?:if|else if)\s*\((?P<condition>.*)\)")


def source_function(lines: list[str], index: int) -> str:
    for cursor in range(index, max(-1, index - 160), -1):
        line = lines[cursor].strip()
        match = FUNCTION_START.match(line)
        if match:
            return match.group("name")
        if line.endswith(")") and cursor + 1 < len(lines) and lines[cursor + 1].strip() == "{":
            candidate = re.search(r"([A-Za-z_]\w*)\s*\(", line)
            if candidate and candidate.group(1) not in {"if", "while", "for", "switch"}:
                return candidate.group(1)
    return "(internal/continued)"


def condition_for(lines: list[str], index: int) -> str:
    current = lines[index].strip()
    inline = re.search(r"\bif\s*\((.*?)\)\s*return\b", current)
    if inline:
        return inline.group(1).strip()
    for cursor in range(index - 1, max(-1, index - 8), -1):
        text = lines[cursor].strip()
        match = CHECK_PREFIX.match(text)
        if match:
            condition = match.group("condition").strip()
            if condition.endswith(")"):
                condition = condition[:-1].rstrip()
            return condition
        if text and not text.startswith(("/*", "*", "//", "{")):
            break
    return "unconditional rejection on this source path"


def md_escape(text: str) -> str:
    return text.replace("|", "\\|").replace("`", "'").strip()


def write_errors() -> None:
    rows: list[tuple[str, str, str, str]] = []
    sources = sorted((C_ROOT / "src").glob("*.[ch]"))
    for source in sources:
        lines = source.read_text(errors="replace").splitlines()
        for index, line in enumerate(lines):
            return_match = ERROR_RETURN.search(line)
            assert_match = ASSERTION.search(line)
            if return_match:
                result = return_match.group("result")
                condition = condition_for(lines, index)
                location = f"{source.relative_to(C_ROOT)}:{index + 1}"
                rows.append(
                    (
                        source_function(lines, index),
                        f"`{md_escape(condition)}` ({location})",
                        f"`{result}`",
                        location,
                    )
                )
            if assert_match:
                expression = assert_match.group("expr").strip()
                location = f"{source.relative_to(C_ROOT)}:{index + 1}"
                rows.append(
                    (
                        source_function(lines, index),
                        f"assertion false: `{md_escape(expression)}` ({location})",
                        "C assertion failure",
                        location,
                    )
                )
    lines = [
        "# Error surface",
        "",
        "Mechanically inventoried from explicit C error/sentinel returns and assertions.",
        "The source location disambiguates repeated conditions and multi-branch returns.",
        "",
        "| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |",
        "|---:|----------|---------------------------------------------|-------------------|-----|",
    ]
    for number, (function, trigger, result, _location) in enumerate(rows, 1):
        lines.append(
            f"| {number} | `{md_escape(function)}` | {trigger} | {result} | [ ] |"
        )
    (CRATE / "ERRORS.md").write_text("\n".join(lines) + "\n")


DEFINE = re.compile(r"^#define\s+(PCRE2_[A-Z0-9_]+)\s+([^/\s][^/]*)")


def public_symbols() -> list[str]:
    return [s for s in dynamic_symbols(C_SO) if s.startswith("pcre2_")]


def constants(prefixes: tuple[str, ...]) -> list[tuple[str, str]]:
    found: list[tuple[str, str]] = []
    for line in HEADER.read_text().splitlines():
        match = DEFINE.match(line)
        if match and match.group(1).startswith(prefixes):
            found.append((match.group(1), match.group(2).strip()))
    return found


def write_configs() -> None:
    rows: list[tuple[str, str]] = []

    # Every public entry point gets a baseline valid shape row.
    for symbol in public_symbols():
        rows.append((symbol, "baseline valid call; null/default context where permitted"))

    # Mechanically enumerate public option families that select C branches.
    families = [
        (
            "pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8",
            (
                "PCRE2_ALLOW_",
                "PCRE2_ALT_",
                "PCRE2_AUTO_",
                "PCRE2_CASELESS",
                "PCRE2_DOLLAR_",
                "PCRE2_DOTALL",
                "PCRE2_DUPNAMES",
                "PCRE2_EXTENDED",
                "PCRE2_FIRSTLINE",
                "PCRE2_MATCH_",
                "PCRE2_MULTILINE",
                "PCRE2_NEVER_",
                "PCRE2_NO_AUTO_",
                "PCRE2_NO_DOTSTAR_",
                "PCRE2_NO_START_",
                "PCRE2_UCP",
                "PCRE2_UNGREEDY",
                "PCRE2_UTF",
                "PCRE2_USE_OFFSET_",
                "PCRE2_LITERAL",
                "PCRE2_ANCHORED",
                "PCRE2_ENDANCHORED",
            ),
        ),
        (
            "pcre2_set_compile_extra_options_8 + pcre2_compile_8",
            ("PCRE2_EXTRA_",),
        ),
        (
            "pcre2_match_8, pcre2_dfa_match_8",
            (
                "PCRE2_NOTBOL",
                "PCRE2_NOTEOL",
                "PCRE2_NOTEMPTY",
                "PCRE2_PARTIAL_",
                "PCRE2_DFA_",
                "PCRE2_NO_JIT",
                "PCRE2_COPY_MATCHED_",
            ),
        ),
        ("pcre2_jit_compile_8, pcre2_jit_match_8", ("PCRE2_JIT_",)),
        ("pcre2_substitute_8", ("PCRE2_SUBSTITUTE_",)),
        ("pcre2_pattern_convert_8", ("PCRE2_CONVERT_",)),
        ("pcre2_config_8", ("PCRE2_CONFIG_",)),
        ("pcre2_pattern_info_8", ("PCRE2_INFO_",)),
        ("pcre2_set_newline_8 + compile/match", ("PCRE2_NEWLINE_",)),
        ("pcre2_set_bsr_8 + compile/match", ("PCRE2_BSR_",)),
    ]
    seen: set[tuple[str, str]] = set()
    for entry_points, prefixes in families:
        for name, value in constants(prefixes):
            item = (entry_points, f"`{name}` = `{md_escape(value)}`")
            if item not in seen:
                rows.append(item)
                seen.add(item)

    # Input shapes explicitly distinguished by public APIs.
    shapes = [
        ("pcre2_compile_8", "pattern length: zero / one / many / PCRE2_ZERO_TERMINATED"),
        ("pcre2_compile_8", "pattern bytes: ASCII / UTF-8 multibyte / embedded NUL"),
        ("pcre2_compile_8", "captures: zero / one / many; named / duplicate-named"),
        ("pcre2_match_8", "subject length: zero / one / many; start offset 0 / middle / end"),
        ("pcre2_match_8", "result shape: no match / empty match / one capture / many captures"),
        ("pcre2_dfa_match_8", "workspace: minimum / larger; first call / DFA restart"),
        ("pcre2_next_match_8", "previous match: nonempty / empty at start / empty at end"),
        ("pcre2_substitute_8", "replacement: empty / literal / numbered ref / named ref / extended"),
        ("pcre2_substitute_8", "output: exact fit / oversized / overflow-length query"),
        ("pcre2_pattern_convert_8", "source: POSIX basic / POSIX extended / glob / UTF"),
        ("pcre2_serialize_encode_8, pcre2_serialize_decode_8", "code count: one / many"),
        ("pcre2_substring_*_8", "capture: set / unset / empty; by number / by name"),
        ("pcre2_get_error_message_8", "buffer: exact fit / oversized"),
        ("pcre2_callout_enumerate_8", "callouts: none / numeric / string / automatic"),
        ("pcre2_jit_stack_create_8", "stack sizes: start equals max / start below max"),
        ("context create/copy/free APIs", "default allocator / custom allocator; original / copy"),
    ]
    rows.extend(shapes)

    lines = [
        "# Configuration surface",
        "",
        "Mechanically based on every exported public entry point and every public option",
        "constant family in `pcre2.h`, plus input shapes explicitly distinguished by the APIs.",
        "",
        "| # | entry point(s) | configuration (options set + input shape) | [ ] |",
        "|---:|----------------|--------------------------------------------|-----|",
    ]
    for number, (entry_points, configuration) in enumerate(rows, 1):
        lines.append(
            f"| {number} | `{md_escape(entry_points)}` | {configuration} | [ ] |"
        )
    (CRATE / "CONFIGS.md").write_text("\n".join(lines) + "\n")


def main() -> None:
    write_symbols()
    write_errors()
    write_configs()


if __name__ == "__main__":
    main()
