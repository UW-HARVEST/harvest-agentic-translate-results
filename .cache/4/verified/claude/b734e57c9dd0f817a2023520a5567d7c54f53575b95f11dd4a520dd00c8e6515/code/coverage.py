#!/usr/bin/env python3
"""Fold the coverage tags emitted by the differential tests back into
CONFIGS.md and ERRORS.md, and report what is still unchecked.

The check-boxes are therefore derived from what actually RAN and PASSED (a
failing test aborts before/at its assertion, and the suite must be green for
the tags to be trusted), never from a claim in a comment.

  ./coverage.py            # update both tables in place, print a summary
  ./coverage.py --check    # exit non-zero if any row is uncovered
"""
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
TAGDIR = os.path.join(ROOT, "target", "difftest-coverage")


def load_tags():
    cfg, err = set(), set()
    for f in glob.glob(os.path.join(TAGDIR, "*.txt")):
        for line in open(f):
            t = line.strip()
            if t.startswith("CFG:"):
                spec = t[4:]
                if "-" in spec:
                    a, b = spec.split("-", 1)
                    try:
                        cfg.update(range(int(a), int(b) + 1))
                    except ValueError:
                        pass
                else:
                    try:
                        cfg.add(int(spec))
                    except ValueError:
                        pass
            elif t.startswith("ERR:"):
                err.add(t[4:].strip())
    return cfg, err


def split_row(line):
    parts = re.split(r"(?<!\\)\|", line.rstrip("\n"))
    return parts


def update_configs(covered):
    path = os.path.join(ROOT, "CONFIGS.md")
    out, total, done = [], 0, 0
    uncovered = []
    for line in open(path):
        p = split_row(line)
        body = [c.strip() for c in p[1:-1]] if line.startswith("|") else []
        if len(body) == 5 and body[0].isdigit():
            n = int(body[0])
            total += 1
            mark = "[x]" if n in covered else "[ ]"
            if n in covered:
                done += 1
            else:
                uncovered.append(n)
            body[4] = mark
            line = "| " + " | ".join(body) + " |\n"
        out.append(line)
    open(path, "w").writelines(out)
    return total, done, uncovered


def update_errors(covered_sites):
    """ERRORS.md rows are keyed by the `file:line` cell (column 2)."""
    path = os.path.join(ROOT, "ERRORS.md")
    out, total, done = [], 0, 0
    uncovered = []
    for line in open(path):
        p = split_row(line)
        body = [c.strip() for c in p[1:-1]] if line.startswith("|") else []
        if len(body) >= 7 and body[0].isdigit():
            total += 1
            site = body[1].strip("`")
            reach = body[6]
            # a site cell may name several lines, e.g. "zstd_lazy.c:1778, :2136"
            base = site.split(":")[0]
            keys = {site}
            for extra in site.split(","):
                extra = extra.strip()
                if extra.startswith(":"):
                    keys.add(base + extra)
                elif extra:
                    keys.add(extra)
            hit = bool(keys & covered_sites)
            # A row is legitimately EXCLUDED from differential testing when the
            # reference C has no defined behaviour to match, or when nothing a
            # public caller can do reaches it. Each such row carries its
            # justification in the `reach` cell.
            excluded = (
                "UNREACHABLE" in reach          # dominated by an earlier check
                or "UNSAFE-UB" in reach         # the C is undefined on this input
                or "ALLOC-ONLY" in reach        # only an internal allocation
                                                # failure reaches it, and there is
                                                # no public injection point
                or "UNSAFE-UB" in reach
            )
            if hit:
                done += 1
                body[6] = reach if "[x]" in reach else reach + " **[x]**"
            elif excluded:
                done += 1
                body[6] = reach if "[excluded]" in reach else reach + " **[excluded]**"
            else:
                uncovered.append((body[0], site, body[2]))
            line = "| " + " | ".join(body) + " |\n"
        out.append(line)
    open(path, "w").writelines(out)
    return total, done, uncovered


def main():
    cfg, err = load_tags()
    if not cfg and not err:
        print(f"no coverage tags found under {TAGDIR}", file=sys.stderr)
        print("run the differential suite first (./run_difftests.sh)", file=sys.stderr)
    ct, cd, cu = update_configs(cfg)
    et, ed, eu = update_errors(err)
    print("CONFIGS.md : %d/%d rows covered" % (cd, ct))
    if cu:
        print("  uncovered rows: %s" % ", ".join(map(str, cu[:60])))
        if len(cu) > 60:
            print("  ... and %d more" % (len(cu) - 60))
    print("ERRORS.md  : %d/%d rows covered or explicitly excluded" % (ed, et))
    if eu:
        print("  uncovered rows:")
        for n, site, trig in eu[:60]:
            print("    #%s %s  %s" % (n, site, trig[:90]))
        if len(eu) > 60:
            print("    ... and %d more" % (len(eu) - 60))
    if "--check" in sys.argv and (cu or eu):
        sys.exit(1)


if __name__ == "__main__":
    main()
