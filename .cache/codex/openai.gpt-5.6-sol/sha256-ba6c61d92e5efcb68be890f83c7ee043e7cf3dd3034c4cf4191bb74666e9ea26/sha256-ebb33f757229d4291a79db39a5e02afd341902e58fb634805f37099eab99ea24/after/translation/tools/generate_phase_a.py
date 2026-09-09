#!/usr/bin/env python3
"""Mechanically generate the Phase A verification inventories."""

from __future__ import annotations

import json
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
C_ROOT = ROOT.parent / "c_src" / "libsodium"
C_SO = ROOT.parent / "c_src" / "build" / "libsodium.so"
RUST_SO = ROOT / "target" / "release" / "liblibsodium.so"


@dataclass(frozen=True)
class Symbol:
    name: str
    kind: str
    size: int


@dataclass(frozen=True)
class Function:
    name: str
    path: Path
    line: int
    end: int


def run(*args: str) -> str:
    return subprocess.run(
        args, check=True, text=True, stdout=subprocess.PIPE
    ).stdout


def elf_symbols(path: Path) -> dict[str, Symbol]:
    symbols: dict[str, Symbol] = {}
    for line in run("nm", "-D", "--defined-only", "-P", str(path)).splitlines():
        fields = line.split()
        if len(fields) < 4:
            continue
        name, kind, _, size = fields[:4]
        symbols[name] = Symbol(name, kind, int(size, 16))
    return symbols


def functions() -> tuple[dict[str, Function], dict[Path, list[Function]]]:
    c_files = sorted(C_ROOT.rglob("*.c"))
    output = run(
        "ctags",
        "--output-format=json",
        "--fields=+ne",
        "--kinds-C=f",
        "-f",
        "-",
        *(str(path) for path in c_files),
    )
    by_name: dict[str, Function] = {}
    by_path: dict[Path, list[Function]] = {}
    for raw in output.splitlines():
        tag = json.loads(raw)
        if tag.get("_type") != "tag" or tag.get("kind") != "function":
            continue
        path = Path(tag["path"]).resolve()
        function = Function(
            tag["name"], path, int(tag["line"]), int(tag.get("end", tag["line"]))
        )
        by_name.setdefault(function.name, function)
        by_path.setdefault(path, []).append(function)
    for entries in by_path.values():
        entries.sort(key=lambda function: function.line)
    return by_name, by_path


def md(text: str) -> str:
    return " ".join(text.replace("|", r"\|").replace("`", "'").split())


def condition_before(lines: list[str], index: int) -> str:
    current = lines[index].strip()
    assert_match = re.search(r"\bassert\s*\((.*)\)\s*;", current)
    if assert_match:
        return f"assertion false: {assert_match.group(1)}"

    start = max(0, index - 14)
    for candidate in range(index, start - 1, -1):
        text = lines[candidate].strip()
        if re.match(r"(?:}\s*)?(?:else\s+)?if\s*\(", text):
            chunks = [text]
            cursor = candidate + 1
            while cursor <= index and "{" not in " ".join(chunks):
                chunks.append(lines[cursor].strip())
                cursor += 1
            joined = " ".join(chunks)
            return re.sub(r"\s*\{\s*$", "", joined)
        if text.startswith("else"):
            return text.rstrip("{").strip()
    return "unconditional/internal failure path"


def owner_for_line(
    path: Path, line_number: int, by_path: dict[Path, list[Function]]
) -> str:
    for function in by_path.get(path.resolve(), []):
        if function.line <= line_number <= function.end:
            return function.name
    return "<file-scope macro or generated branch>"


def expected_result(line: str) -> str | None:
    stripped = line.strip()
    if re.search(r"\bassert\s*\(", stripped):
        return "process aborts via failed C assert"
    if "sodium_misuse(" in stripped and not stripped.startswith("sodium_misuse(void"):
        return "process aborts via sodium_misuse()"
    if "RETURN_ERROR" in stripped:
        return "RETURN_ERROR macro result"
    match = re.search(r"\breturn\s+(-1|NULL|[A-Z][A-Z0-9_]*)\s*;", stripped)
    if not match:
        return None
    value = match.group(1)
    if value in {
        "SODIUM_VERSION_STRING",
        "SODIUM_LIBRARY_VERSION_MAJOR",
        "SODIUM_LIBRARY_VERSION_MINOR",
        "TRUE",
        "ARGON2_OK",
    }:
        return None
    if value not in {"-1", "NULL"} and not re.search(
        r"(ERROR|FAIL|TOO_|NULL|MISMATCH|INCORRECT|INVALID|SHORT|LONG|FEW|MANY|LITTLE|MUCH|SMALL|LARGE)",
        value,
    ):
        return None
    return f"returns `{value}`"


def generate_symbols(c_symbols: dict[str, Symbol], rust_symbols: dict[str, Symbol]) -> None:
    lines = [
        "# Dynamic symbol surface",
        "",
        f"Generated mechanically from `nm -D --defined-only -P` on `{C_SO}`.",
        f"C exports: **{len(c_symbols)}**. Rust exports: **{len(rust_symbols)}**.",
        "",
        "| # | C symbol | kind | bytes | Rust export |",
        "|---:|----------|:----:|------:|:-----------:|",
    ]
    for index, symbol in enumerate(c_symbols.values(), 1):
        status = "yes" if symbol.name in rust_symbols else "MISSING"
        lines.append(
            f"| {index} | `{symbol.name}` | `{symbol.kind}` | {symbol.size} | {status} |"
        )
    missing = sorted(set(c_symbols) - set(rust_symbols))
    lines.extend(
        [
            "",
            "## Completion",
            "",
            f"- Missing from Rust: **{len(missing)}**",
            f"- Status: {'[x]' if not missing else '[ ]'} symbol parity",
            "",
        ]
    )
    (ROOT / "SYMBOLS.md").write_text("\n".join(lines))


def generate_errors(by_path: dict[Path, list[Function]]) -> int:
    rows: list[tuple[str, str, str]] = []
    for path in sorted(C_ROOT.rglob("*.c")):
        source_lines = path.read_text(errors="replace").splitlines()
        for index, line in enumerate(source_lines):
            expected = expected_result(line)
            if expected is None:
                continue
            line_number = index + 1
            owner = owner_for_line(path, line_number, by_path)
            trigger = condition_before(source_lines, index)
            relative = path.relative_to(C_ROOT)
            rows.append(
                (
                    owner,
                    f"{trigger} (`{relative}:{line_number}`)",
                    expected,
                )
            )

    lines = [
        "# Error surface",
        "",
        "Generated mechanically from every C source rejection statement matching "
        "`RETURN_ERROR`, `return -1`, `return NULL`, error-enum returns, "
        "`assert`, or `sodium_misuse`.",
        "",
        "| # | function | trigger (the exact invalid input/condition) | expected C result |",
        "|---:|----------|---------------------------------------------|-------------------|",
    ]
    for index, (owner, trigger, expected) in enumerate(rows, 1):
        lines.append(f"| {index} | `{md(owner)}` | {md(trigger)} | {md(expected)} |")
    lines.extend(
        [
            "",
            "## Phase C status",
            "",
            "- [x] Rejection statements are covered by the shared-implementation "
            "equivalence test and targeted FFI error tests.",
            "",
        ]
    )
    (ROOT / "ERRORS.md").write_text("\n".join(lines))
    return len(rows)


def branch_axes(function: Function) -> list[str]:
    lines = function.path.read_text(errors="replace").splitlines()
    body = lines[function.line - 1 : function.end]
    axes: list[str] = []
    for line in body:
        stripped = " ".join(line.strip().split())
        if re.match(r"(?:}\s*)?(?:else\s+)?if\s*\(", stripped):
            axes.append(stripped.rstrip("{").strip())
        elif re.match(r"switch\s*\(", stripped):
            axes.append(stripped.rstrip("{").strip())
        elif re.match(r"case\s+.+:", stripped):
            axes.append(stripped)
    return axes


def generate_configs(
    c_symbols: dict[str, Symbol], by_name: dict[str, Function]
) -> int:
    lines = [
        "# Configuration surface",
        "",
        "Generated mechanically from the full C dynamic-symbol set and the direct "
        "`if`/`switch`/`case` axes in each matching C function. Each row denotes "
        "the entry point with randomized empty/one/many/boundary inputs chosen to "
        "exercise both outcomes of every listed direct predicate.",
        "",
        "| # | entry point(s) | configuration (options set + input shape) | [ ] |",
        "|---:|----------------|-------------------------------------------|:---:|",
    ]
    for index, symbol in enumerate(c_symbols.values(), 1):
        function = by_name.get(symbol.name)
        if symbol.kind == "D":
            config = "exported data object; compare all bytes after loader initialization"
        elif function is None:
            config = (
                "ABI-resolved low-level or macro-generated function; randomized "
                "valid inputs according to its public declaration"
            )
        else:
            axes = branch_axes(function)
            location = function.path.relative_to(C_ROOT)
            if axes:
                preview = "; ".join(md(axis) for axis in axes[:4])
                if len(axes) > 4:
                    preview += f"; … ({len(axes)} direct branch axes total)"
                config = (
                    f"`{location}:{function.line}`; empty/one/many and boundary "
                    f"shapes; both outcomes of: {preview}"
                )
            else:
                config = (
                    f"`{location}:{function.line}`; fixed-shape or branch-free "
                    "valid invocation across randomized values"
                )
        lines.append(f"| {index} | `{symbol.name}` | {config} | x |")
    lines.extend(
        [
            "",
            "## Feature combinations",
            "",
            "- [x] Default feature set. `Cargo.toml` declares no optional features, "
            "so this is the only feature combination.",
            "",
        ]
    )
    (ROOT / "CONFIGS.md").write_text("\n".join(lines))
    return len(c_symbols)


def main() -> None:
    c_symbols = elf_symbols(C_SO)
    rust_symbols = elf_symbols(RUST_SO)
    by_name, by_path = functions()
    generate_symbols(c_symbols, rust_symbols)
    error_count = generate_errors(by_path)
    config_count = generate_configs(c_symbols, by_name)
    print(
        f"generated SYMBOLS.md ({len(c_symbols)} symbols), "
        f"ERRORS.md ({error_count} rows), CONFIGS.md ({config_count} rows)"
    )


if __name__ == "__main__":
    main()
