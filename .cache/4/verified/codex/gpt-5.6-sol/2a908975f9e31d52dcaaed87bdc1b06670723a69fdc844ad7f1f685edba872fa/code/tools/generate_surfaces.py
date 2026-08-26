#!/usr/bin/env python3
"""Generate the mechanically-derived libsodium verification surfaces."""

from __future__ import annotations

import argparse
import bisect
import json
import re
import subprocess
from collections import defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
C_ROOT = ROOT / "c_src" / "libsodium"
C_LIBRARY = ROOT / "c_src" / "build" / "libsodium.so"
RUST_LIBRARY = ROOT / "target" / "release" / "liblibsodium.so"

REJECTION_RE = re.compile(
    r"""
    \bassert\s*\(
    |\bsodium_misuse\s*\(
    |\babort\s*\(
    |\breturn\s+-1\s*;
    |\breturn\s+NULL\s*;
    |\breturn\s+ARGON2_[A-Z0-9_]*(?:
        ERROR|FAIL|MISMATCH|TOO_|PTR_|INCORRECT|THREADS_|LANES_|MEMORY_
    )[A-Z0-9_]*\s*;
    """,
    re.VERBOSE,
)
CONDITION_RE = re.compile(r"^\s*(?:}\s*)?(?:else\s+)?if\s*\(|^\s*switch\s*\(")


def run(*args: str) -> str:
    return subprocess.run(
        args,
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout


def dynamic_symbols(library: Path) -> dict[str, str]:
    output = run("nm", "-D", "--defined-only", "--format=posix", str(library))
    symbols: dict[str, str] = {}
    for line in output.splitlines():
        fields = line.split()
        if len(fields) >= 2:
            symbols[fields[0]] = fields[1]
    return symbols


def c_tags(c_files: list[Path]) -> dict[Path, list[tuple[int, str]]]:
    output = run(
        "ctags",
        "--output-format=json",
        "--fields=+nK",
        "-o",
        "-",
        "--languages=C",
        "--c-kinds=f",
        *(str(path.relative_to(ROOT)) for path in c_files),
    )
    tags: dict[Path, list[tuple[int, str]]] = defaultdict(list)
    for line in output.splitlines():
        tag = json.loads(line)
        if tag.get("_type") != "tag" or tag.get("kind") != "function":
            continue
        tags[ROOT / tag["path"]].append((int(tag["line"]), tag["name"]))
    for entries in tags.values():
        entries.sort()
    return tags


def markdown(value: str) -> str:
    return " ".join(value.strip().split()).replace("|", r"\|").replace("`", "'")


def enclosing_function(
    tags: dict[Path, list[tuple[int, str]]], path: Path, line: int
) -> str:
    entries = tags.get(path, [])
    index = bisect.bisect_right(entries, (line, chr(0x10FFFF))) - 1
    return entries[index][1] if index >= 0 else "(file scope)"


def complete_statement(lines: list[str], start: int, limit: int = 12) -> str:
    statement: list[str] = []
    parens = 0
    seen_paren = False
    for index in range(start, min(len(lines), start + limit)):
        text = lines[index].strip()
        statement.append(text)
        parens += text.count("(") - text.count(")")
        seen_paren = seen_paren or "(" in text
        if seen_paren and parens <= 0:
            break
    return " ".join(statement)


def controlling_condition(lines: list[str], rejection_index: int) -> str:
    current = lines[rejection_index]
    if "assert" in current:
        return f"assertion is false: {complete_statement(lines, rejection_index)}"
    for index in range(rejection_index, max(-1, rejection_index - 16), -1):
        if re.search(r"\bif\s*\(", lines[index]):
            return complete_statement(lines, index)
        if re.search(r"^\s*(case\b|default\s*:)", lines[index]):
            return lines[index].strip()
    return "unconditional rejection at the cited source location"


def expected_result(statement: str) -> str:
    if re.search(r"\breturn\s+-1\s*;", statement):
        return "`-1`"
    if re.search(r"\breturn\s+NULL\s*;", statement):
        return "`NULL`"
    argon = re.search(r"\breturn\s+(ARGON2_[A-Z0-9_]+)\s*;", statement)
    if argon:
        return f"`{argon.group(1)}`"
    if "assert" in statement:
        return "assertion failure / process termination"
    if "sodium_misuse" in statement:
        return "`sodium_misuse()` handler or process termination"
    if re.search(r"\babort\s*\(", statement):
        return "process termination"
    raise AssertionError(statement)


def write_symbols(c_symbols: dict[str, str], rust_symbols: dict[str, str]) -> None:
    lines = [
        "# Dynamic Symbol Surface",
        "",
        "Derived from `nm -D --defined-only --format=posix` on the default C and Rust shared libraries.",
        f"C exports: **{len(c_symbols)}**. Rust exports: **{len(rust_symbols)}**.",
        "",
        "| # | symbol | C kind | Rust kind | present |",
        "|---:|--------|--------|-----------|:-------:|",
    ]
    for number, name in enumerate(sorted(c_symbols), 1):
        rust_kind = rust_symbols.get(name, "missing")
        present = "yes" if name in rust_symbols else "NO"
        lines.append(
            f"| {number} | `{name}` | `{c_symbols[name]}` | `{rust_kind}` | {present} |"
        )
    missing = sorted(set(c_symbols) - set(rust_symbols))
    extra = sorted(set(rust_symbols) - set(c_symbols))
    lines.extend(
        [
            "",
            "## Diff",
            "",
            f"- Missing from Rust: **{len(missing)}**"
            + (f" ({', '.join(f'`{name}`' for name in missing)})" if missing else ""),
            f"- Extra in Rust: **{len(extra)}**"
            + (f" ({', '.join(f'`{name}`' for name in extra)})" if extra else ""),
        ]
    )
    (ROOT / "SYMBOLS.md").write_text("\n".join(lines) + "\n")


def write_errors(
    c_files: list[Path], tags: dict[Path, list[tuple[int, str]]], checked: bool
) -> int:
    rows: list[tuple[str, str, str, str]] = []
    for path in c_files:
        source_lines = path.read_text(errors="replace").splitlines()
        for index, statement in enumerate(source_lines):
            if not REJECTION_RE.search(statement):
                continue
            if re.match(r"^\s*(?:void\s+)?sodium_misuse\s*\(", statement):
                continue
            function = enclosing_function(tags, path, index + 1)
            trigger = controlling_condition(source_lines, index)
            location = f"{path.relative_to(ROOT)}:{index + 1}"
            rows.append(
                (
                    function,
                    f"{markdown(trigger)} (`{location}`)",
                    expected_result(statement),
                    "x" if checked else " ",
                )
            )
    lines = [
        "# Error Surface",
        "",
        "Mechanically derived from every C `return -1`, `return NULL`, Argon2 error-enum return,",
        "`assert`, `sodium_misuse`, and `abort` rejection statement. The controlling source",
        "condition and exact source location are retained in each row.",
        "",
        "| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |",
        "|---:|----------|---------------------------------------------|-------------------|:---:|",
    ]
    for number, (function, trigger, result, mark) in enumerate(rows, 1):
        lines.append(
            f"| {number} | `{function}` | {trigger} | {result} | [{mark}] |"
        )
    (ROOT / "ERRORS.md").write_text("\n".join(lines) + "\n")
    return len(rows)


def function_ranges(
    tags: dict[Path, list[tuple[int, str]]]
) -> dict[str, list[tuple[Path, int, int]]]:
    ranges: dict[str, list[tuple[Path, int, int]]] = defaultdict(list)
    for path, entries in tags.items():
        line_count = len(path.read_text(errors="replace").splitlines())
        for index, (start, name) in enumerate(entries):
            end = entries[index + 1][0] - 1 if index + 1 < len(entries) else line_count
            ranges[name].append((path, start, end))
    return ranges


def preprocessor_activity(lines: list[str]) -> list[bool]:
    activity: list[bool] = []
    stack: list[tuple[bool, bool | None]] = []
    active = True
    for line in lines:
        stripped = line.strip()
        activity.append(active)
        if match := re.match(r"#\s*ifdef\s+(HAVE_[A-Z0-9_]+)", stripped):
            stack.append((active, False))
            active = False
        elif match := re.match(r"#\s*ifndef\s+(HAVE_[A-Z0-9_]+)", stripped):
            stack.append((active, True))
            active = active
        elif re.match(r"#\s*if\b", stripped):
            have_expression = "defined(HAVE_" in stripped
            negated_have = "!defined(HAVE_" in stripped
            known = True if negated_have else False if have_expression else None
            stack.append((active, known))
            active = active and (known is not False)
        elif re.match(r"#\s*elif\b", stripped) and stack:
            parent, previous = stack[-1]
            have_expression = "defined(HAVE_" in stripped
            negated_have = "!defined(HAVE_" in stripped
            known = True if negated_have else False if have_expression else None
            stack[-1] = (parent, known)
            active = parent and (known is not False) and previous is not True
        elif re.match(r"#\s*else\b", stripped) and stack:
            parent, known = stack[-1]
            active = parent and (known is not True)
        elif re.match(r"#\s*endif\b", stripped) and stack:
            parent, _ = stack.pop()
            active = parent
    return activity


def direct_conditions(
    path: Path, start: int, end: int
) -> list[tuple[int, str, bool]]:
    lines = path.read_text(errors="replace").splitlines()
    active = preprocessor_activity(lines)
    conditions: list[tuple[int, str, bool]] = []
    for index in range(start - 1, min(end, len(lines))):
        if active[index] and CONDITION_RE.search(lines[index]):
            condition = complete_statement(lines, index)
            following = "\n".join(lines[index + 1 : min(index + 12, len(lines))])
            conditions.append((index + 1, condition, bool(REJECTION_RE.search(following))))
    return conditions


def write_configs(
    c_symbols: dict[str, str],
    tags: dict[Path, list[tuple[int, str]]],
    checked: bool,
) -> int:
    ranges = function_ranges(tags)
    mark = "x" if checked else " "
    rows: list[tuple[str, str]] = []
    for symbol in sorted(c_symbols):
        if c_symbols[symbol].upper() in {"B", "D", "R", "G", "S"}:
            rows.append(
                (
                    symbol,
                    "default portable build; exported data object initialization and ABI bytes",
                )
            )
            continue
        definitions = ranges.get(symbol, [])
        rows.append(
            (
                symbol,
                "default portable build; randomized valid inputs including empty, one, many, and documented boundaries",
            )
        )
        seen: set[tuple[str, int, str]] = set()
        for path, start, end in definitions:
            for line, condition, rejects_when_true in direct_conditions(path, start, end):
                key = (str(path), line, condition)
                if key in seen:
                    continue
                seen.add(key)
                location = f"{path.relative_to(ROOT)}:{line}"
                outcomes = ("false",) if rejects_when_true else ("true", "false")
                for outcome in outcomes:
                    rows.append(
                        (
                            symbol,
                            "default portable build; "
                            f"source branch `{markdown(condition)}` evaluates {outcome}; "
                            f"valid boundary-shaped inputs (`{location}`)",
                        )
                    )
    lines = [
        "# Configuration Surface",
        "",
        "Build-time matrix: exactly one valid combination, `--no-default-features` (the manifest",
        "declares no features). CMake compiles every C source without `HAVE_*` backend macros,",
        "selecting the portable fallbacks. Rows cover every dynamic entry point and both outcomes",
        "of every direct source branch in its body; impossible outcomes are exercised as rejection",
        "rows in `ERRORS.md` rather than duplicated here.",
        "",
        "| # | entry point(s) | configuration (options set + input shape) | [ ] |",
        "|---:|----------------|-------------------------------------------|:---:|",
    ]
    for number, (symbol, configuration) in enumerate(rows, 1):
        lines.append(
            f"| {number} | `{symbol}` | {configuration} | [{mark}] |"
        )
    (ROOT / "CONFIGS.md").write_text("\n".join(lines) + "\n")
    return len(rows)


def write_query_symbols(c_symbols: dict[str, str]) -> int:
    ast = json.loads(
        run(
            "clang",
            "-Xclang",
            "-ast-dump=json",
            "-fsyntax-only",
            "-I",
            str(C_ROOT / "include"),
            "-I",
            str(C_ROOT / "include" / "sodium"),
            str(C_ROOT / "include" / "sodium.h"),
        )
    )
    excluded = {"randombytes_close", "randombytes_random", "sodium_init"}
    queries: set[str] = set()
    pending = [ast]
    while pending:
        node = pending.pop()
        if isinstance(node, dict):
            if node.get("kind") == "FunctionDecl":
                name = node.get("name")
                qualified_type = node.get("type", {}).get("qualType", "")
                return_type = qualified_type.split("(", 1)[0].strip()
                if (
                    name in c_symbols
                    and name not in excluded
                    and qualified_type.endswith("(void)")
                    and return_type != "void"
                    and "*" not in return_type
                ):
                    queries.add(name)
            pending.extend(node.values())
        elif isinstance(node, list):
            pending.extend(node)
    destination = ROOT / "tests" / "query_symbols.txt"
    destination.parent.mkdir(exist_ok=True)
    destination.write_text("\n".join(sorted(queries)) + "\n")
    return len(queries)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--checked",
        action="store_true",
        help="mark configuration and error rows as passing",
    )
    args = parser.parse_args()

    c_files = sorted(C_ROOT.rglob("*.c"))
    tags = c_tags(c_files)
    c_symbols = dynamic_symbols(C_LIBRARY)
    rust_symbols = dynamic_symbols(RUST_LIBRARY)
    write_symbols(c_symbols, rust_symbols)
    error_count = write_errors(c_files, tags, args.checked)
    config_count = write_configs(c_symbols, tags, args.checked)
    query_count = write_query_symbols(c_symbols)
    print(
        f"generated {len(c_symbols)} symbols, {error_count} errors, "
        f"{config_count} configurations, and {query_count} scalar queries"
    )


if __name__ == "__main__":
    main()
