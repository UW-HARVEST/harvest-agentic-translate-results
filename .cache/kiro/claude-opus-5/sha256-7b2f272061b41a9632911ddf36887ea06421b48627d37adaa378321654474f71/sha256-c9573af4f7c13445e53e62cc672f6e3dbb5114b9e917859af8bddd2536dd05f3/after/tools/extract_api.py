#!/usr/bin/env python3
"""Mechanically extract every SODIUM_EXPORT declaration from the public headers."""
import re, os, sys, json

ROOT = "c_src/libsodium/include/sodium"
decls = []
for fn in sorted(os.listdir(ROOT)):
    if not fn.endswith(".h"):
        continue
    path = os.path.join(ROOT, fn)
    src = open(path, encoding="utf-8", errors="replace").read()
    # strip comments
    src = re.sub(r"/\*.*?\*/", " ", src, flags=re.S)
    src = re.sub(r"//[^\n]*", " ", src)
    # every SODIUM_EXPORT is followed by a declaration terminated by ';'
    for m in re.finditer(r"SODIUM_EXPORT\s+(.*?);", src, flags=re.S):
        d = " ".join(m.group(1).split())
        # drop trailing attributes
        d = re.sub(r"__attribute__\s*\(\(.*?\)\)\s*$", "", d).strip()
        d = re.sub(r"\s*__attribute__\s*\(\([^()]*(\([^()]*\))?[^()]*\)\)", "", d).strip()
        # name = identifier immediately before the first '('
        nm = re.search(r"([A-Za-z_][A-Za-z0-9_]*)\s*\(", d)
        if not nm:
            # a variable declaration, e.g. extern const char *foo;
            nm2 = re.search(r"([A-Za-z_][A-Za-z0-9_]*)\s*$", d)
            decls.append({"header": fn, "name": nm2.group(1) if nm2 else "?",
                          "decl": d, "kind": "var"})
            continue
        name = nm.group(1)
        ret = d[: nm.start(1)].strip()
        args = d[nm.end(1):].strip()
        # balanced parameter list
        depth = 0
        end = None
        for i, ch in enumerate(args):
            if ch == "(":
                depth += 1
            elif ch == ")":
                depth -= 1
                if depth == 0:
                    end = i
                    break
        params = args[1:end] if end is not None else args
        params = " ".join(params.split())
        decls.append({"header": fn, "name": name, "ret": ret,
                      "params": params, "decl": d, "kind": "fn"})

json.dump(decls, open("/tmp/api.json", "w"), indent=1)
fns = [d for d in decls if d["kind"] == "fn"]
print("declarations:", len(decls), " functions:", len(fns))
zero = [d for d in fns if d["params"] in ("void", "")]
print("zero-arg:", len(zero))
from collections import Counter
print(Counter(d["ret"] for d in zero))
