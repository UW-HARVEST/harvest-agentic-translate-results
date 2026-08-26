#!/usr/bin/env python3
"""Mechanically derive the C symbol, error, and configuration surfaces."""

from __future__ import annotations

import json
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
C_ROOT = ROOT / "c_src" / "src"
C_SO = ROOT / "c_src" / "build" / "libzstd.so"
RUST_SO = ROOT / "target" / "release" / "libzstd.so"


@dataclass(frozen=True)
class Function:
    name: str
    path: Path
    line: int
    end: int
    signature: str
    parameters: frozenset[str]


def run(*args: str) -> str:
    return subprocess.run(args, check=True, text=True, capture_output=True).stdout


def dynamic_symbols(path: Path) -> list[str]:
    output = run("nm", "-D", "--defined-only", str(path))
    return sorted({line.split()[-1] for line in output.splitlines() if line.split()})


def source_files() -> list[Path]:
    return sorted(C_ROOT.rglob("*.c"))


def squash(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip()


def markdown(text: str) -> str:
    return squash(text).replace("|", r"\|").replace("`", r"\`")


def statement(lines: list[str], index: int, limit: int = 12) -> str:
    parts: list[str] = []
    depth = 0
    for line in lines[index : index + limit]:
        stripped = line.strip()
        if stripped:
            parts.append(stripped)
        depth += stripped.count("(") + stripped.count("{")
        depth -= stripped.count(")") + stripped.count("}")
        if ";" in stripped and depth <= 0:
            break
        if stripped.endswith("}") and depth <= 0:
            break
    return squash(" ".join(parts))


def parameter_names(signature: str) -> frozenset[str]:
    if "(" not in signature:
        return frozenset()
    inner = signature[signature.find("(") + 1 : signature.rfind(")")]
    names: set[str] = set()
    for item in inner.split(","):
        item = re.sub(r"/\*.*?\*/", "", item).strip()
        match = re.search(r"([A-Za-z_][A-Za-z0-9_]*)\s*(?:\[[^\]]*\])?\s*$", item)
        if match and match.group(1) != "void":
            names.add(match.group(1))
    return frozenset(names)


def functions() -> list[Function]:
    command = [
        "ctags",
        "--output-format=json",
        "--fields=+neK",
        "--kinds-C=f",
        "-o",
        "-",
        *map(str, source_files()),
    ]
    tags: list[Function] = []
    for raw in run(*command).splitlines():
        tag = json.loads(raw)
        if tag.get("kind") != "function":
            continue
        path = Path(tag["path"])
        lines = path.read_text(errors="replace").splitlines()
        start = int(tag["line"])
        end = int(tag.get("end", start))
        first = max(0, start - 1)
        signature = statement(lines, first, 20)
        tags.append(
            Function(
                tag["name"],
                path,
                start,
                end,
                signature,
                parameter_names(signature),
            )
        )
    return tags


def containing_function(tags: list[Function], path: Path, line: int) -> Function | None:
    candidates = [tag for tag in tags if tag.path == path and tag.line <= line <= tag.end]
    return max(candidates, key=lambda tag: tag.line, default=None)


def write_symbols(c_symbols: list[str], rust_symbols: list[str]) -> None:
    missing = sorted(set(c_symbols) - set(rust_symbols))
    extra = sorted(set(rust_symbols) - set(c_symbols))
    rows = [
        "# Dynamic Symbol Surface",
        "",
        f"- C defined dynamic symbols: {len(c_symbols)}",
        f"- Rust defined dynamic symbols: {len(rust_symbols)}",
        f"- Missing from Rust: {len(missing)}",
        f"- Extra in Rust: {len(extra)}",
        "",
        "| # | C symbol | Rust export |",
        "|---:|----------|-------------|",
    ]
    rust = set(rust_symbols)
    rows.extend(
        f"| {number} | `{symbol}` | {'yes' if symbol in rust else 'MISSING'} |"
        for number, symbol in enumerate(c_symbols, 1)
    )
    (ROOT / "SYMBOLS.md").write_text("\n".join(rows) + "\n")


def rejection_rows(tags: list[Function]) -> list[tuple[str, str, str]]:
    macro_or_assert = re.compile(
        r"\b(?:RETURN_ERROR(?:_IF)?|FORWARD_IF_ERROR|assert|ZSTD_STATIC_ASSERT)\s*\("
    )
    error_return = re.compile(
        r"\breturn\s+(?:NULL|0\s*-\s*ERROR\s*\(|ERROR\s*\(|"
        r"\(?\s*size_t\s*\)?\s*-\s*[A-Za-z_]|-1\b|"
        r"[A-Za-z0-9_]*(?:error|Error)[A-Za-z0-9_]*)"
    )
    rows: list[tuple[str, str, str]] = []
    seen: set[tuple[str, int, str]] = set()
    for path in source_files():
        lines = path.read_text(errors="replace").splitlines()
        for index, line in enumerate(lines):
            if not (macro_or_assert.search(line) or error_return.search(line)):
                continue
            text = statement(lines, index)
            key = (str(path), index + 1, text)
            if key in seen:
                continue
            seen.add(key)
            function = containing_function(tags, path, index + 1)
            name = function.name if function else "<file scope/macro>"
            relative = path.relative_to(ROOT)
            result = "assertion/abort" if re.search(r"\bassert\s*\(", text) else "exact return/error shown"
            rows.append((name, f"`{markdown(text)}` ({relative}:{index + 1})", result))
    return rows


def write_errors(tags: list[Function]) -> None:
    rows = rejection_rows(tags)
    output = [
        "# Error Surface",
        "",
        "Mechanically extracted from C rejection macros, error/sentinel returns, and assertions.",
        "",
        "| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |",
        "|---:|----------|---------------------------------------------|-------------------|-----|",
    ]
    output.extend(
        f"| {number} | `{name}` | {trigger} | {result} | [ ] |"
        for number, (name, trigger, result) in enumerate(rows, 1)
    )
    (ROOT / "ERRORS.md").write_text("\n".join(output) + "\n")


def branch_conditions(lines: list[str], function: Function) -> list[str]:
    conditions: list[str] = []
    parameters = function.parameters
    if not parameters:
        return conditions
    for index in range(function.line - 1, min(function.end, len(lines))):
        text = statement(lines, index, 8)
        if not re.match(r"^(?:if|else if|switch)\s*\(", text):
            continue
        if not any(re.search(rf"\b{re.escape(parameter)}\b", text) for parameter in parameters):
            continue
        condition = text
        if condition not in conditions:
            conditions.append(condition)
    return conditions


def write_configs(tags: list[Function], symbols: list[str]) -> None:
    by_name: dict[str, list[Function]] = {}
    for function in tags:
        by_name.setdefault(function.name, []).append(function)
    rows: list[tuple[str, str]] = []
    for symbol in symbols:
        matches = by_name.get(symbol, [])
        if not matches:
            rows.append((symbol, "exported macro/generated entry point; direct valid path"))
            continue
        function = matches[0]
        lines = function.path.read_text(errors="replace").splitlines()
        conditions = branch_conditions(lines, function)
        location = f"{function.path.relative_to(ROOT)}:{function.line}"
        if not conditions:
            rows.append((symbol, f"direct valid path ({location})"))
            continue
        for condition in conditions:
            rendered = markdown(condition)
            rows.append((symbol, f"`{rendered}` is false ({location})"))
            rows.append((symbol, f"`{rendered}` is true ({location})"))
    output = [
        "# Configuration Surface",
        "",
        "## Build-Time Configurations",
        "",
        "- Cargo declares no `[features]`; the only valid feature combination is",
        "  `--no-default-features --features ''` (equivalent to the default build).",
        "- CMake builds every `src/{common,compress,decompress,dictBuilder,deprecated,legacy}/*.c`.",
        "- C compile definitions: `ZSTD_LEGACY_SUPPORT=5`, `XXH_NAMESPACE=ZSTD_`,",
        "  and `DYNAMIC_BMI2=0`.",
        "",
        "## Runtime Configurations",
        "",
        "Mechanically enumerated from every C dynamic entry point and both outcomes of",
        "each branch/switch in its body that references an API parameter.",
        "",
        "| # | entry point(s) | configuration (options set + input shape) | [ ] |",
        "|---:|----------------|--------------------------------------------|-----|",
    ]
    output.extend(
        f"| {number} | `{name}` | {configuration} | [ ] |"
        for number, (name, configuration) in enumerate(rows, 1)
    )
    (ROOT / "CONFIGS.md").write_text("\n".join(output) + "\n")


def main() -> None:
    if not C_SO.exists() or not RUST_SO.exists():
        raise SystemExit("build both shared libraries before generating surfaces")
    c_symbols = dynamic_symbols(C_SO)
    rust_symbols = dynamic_symbols(RUST_SO)
    tags = functions()
    write_symbols(c_symbols, rust_symbols)
    write_errors(tags)
    write_configs(tags, c_symbols)


if __name__ == "__main__":
    main()
