import subprocess, sys, os, itertools
BASE = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
C = BASE + "/c_src/build/driver"
R = BASE + "/translation/target/release/driver"
def run(exe, data):
    p = subprocess.run([exe], input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return p.stdout, p.stderr, p.returncode
AL = b"0.exni-f9"
fails = n = 0
for combo in itertools.product(AL, repeat=4):
    t = bytes(combo)
    data = t + b"\n" + t + b"\n"
    n += 1
    a, b = run(C, data), run(R, data)
    if a != b:
        fails += 1
        print("FAIL %r\n  C: %r\n  R: %r" % (data, a, b), flush=True)
        if fails > 20: break
print("%d/%d failed" % (fails, n))
