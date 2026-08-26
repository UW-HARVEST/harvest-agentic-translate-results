import sys, os, subprocess
from concurrent.futures import ThreadPoolExecutor
import gen_cases
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
C_BIN = os.path.join(ROOT, "c_src", "build", "driver")
R_BIN = os.path.join(ROOT, "target", "release", "driver")

def one(item):
    data, label = item
    try:
        c = subprocess.run([C_BIN], input=data, stdout=subprocess.PIPE,
                           stderr=subprocess.DEVNULL, timeout=20)
        r = subprocess.run([R_BIN], input=data, stdout=subprocess.PIPE,
                           stderr=subprocess.DEVNULL, timeout=20)
    except subprocess.TimeoutExpired:
        return f"TIMEOUT {label!r}"
    if c.stdout != r.stdout or c.returncode != r.returncode:
        return (f"INPUT {label!r} ({data!r})\n"
                f"  C:    rc={c.returncode} {c.stdout!r}\n"
                f"  RUST: rc={r.returncode} {r.stdout!r}")
    return None

n = int(sys.argv[1]) if len(sys.argv) > 1 else 4000
seed = int(sys.argv[2], 0) if len(sys.argv) > 2 else 0xC0FFEE
cases = gen_cases.all_cases(seed=seed, n=n)
fails = []
with ThreadPoolExecutor(max_workers=32) as ex:
    for res in ex.map(one, cases):
        if res:
            fails.append(res)
print(f"ran {len(cases)} cases, {len(fails)} failures")
for f in fails[:60]:
    print(f)
sys.exit(1 if fails else 0)
