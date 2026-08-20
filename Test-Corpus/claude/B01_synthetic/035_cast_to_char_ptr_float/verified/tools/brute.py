import sys, os, subprocess, itertools
from concurrent.futures import ThreadPoolExecutor
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
C_BIN = os.path.join(ROOT, "c_src", "build", "driver")
R_BIN = os.path.join(ROOT, "target", "release", "driver")

def one(data):
    c = subprocess.run([C_BIN], input=data, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    r = subprocess.run([R_BIN], input=data, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    if c.stdout != r.stdout or c.returncode != r.returncode:
        return f"{data!r}: C={c.stdout!r} rc={c.returncode} RUST={r.stdout!r} rc={r.returncode}"
    return None

alpha = sys.argv[1]
maxlen = int(sys.argv[2])
cases = []
for L in range(1, maxlen+1):
    for t in itertools.product(alpha, repeat=L):
        cases.append("".join(t).encode())
print(f"alphabet={alpha!r} maxlen={maxlen} cases={len(cases)}", flush=True)
fails=[]
with ThreadPoolExecutor(max_workers=48) as ex:
    for res in ex.map(one, cases):
        if res: fails.append(res)
print(f"failures={len(fails)}")
for f in fails[:60]: print(f)
