#!/usr/bin/env python3
import subprocess, sys, os, random
BASE = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
C = os.path.join(BASE, "c_src/build/driver")
R = os.path.join(BASE, "translation/target/release/driver")

def run(exe, data):
    p = subprocess.run([exe], input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return p.stdout, p.stderr, p.returncode

random.seed(int(sys.argv[1]) if len(sys.argv) > 1 else 0)
ALPHA = b"0123456789.eE+-xXaAbBcCdDfFpPnN iItTyY\t\r\x0b\x0c\x00 \n"

def gen_token():
    mode = random.randrange(10)
    if mode == 0:
        return random.choice([b"inf", b"INF", b"infinity", b"nan", b"NaN", b"-inf",
                              b"nan(1)", b"INFI", b"iNf"])
    if mode == 1:
        return b"0x" + bytes(random.choice(b"0123456789abcdefABCDEF.pP+-")
                             for _ in range(random.randrange(0, 10)))
    if mode == 2:
        return b"%de%d" % (random.randrange(-10**6, 10**6), random.randrange(-350, 350))
    if mode == 3:
        return b"%.17g" % random.uniform(-1e-5, 1e-5)
    if mode == 4:
        return b"%.17g" % random.uniform(-1e6, 1e6)
    if mode == 5:  # near the 1e-6 epsilon boundary
        return b"%.17g" % (1e-6 * random.uniform(0.9999, 1.0001) * random.choice([1, -1]))
    if mode == 6:  # near int overflow boundary of 100.0/x
        return b"%.17g" % (100.0 / 2147483647.0 * random.uniform(0.999, 1.001)
                           * random.choice([1, -1]))
    if mode == 7:
        return bytes(random.choice(ALPHA) for _ in range(random.randrange(0, 30)))
    if mode == 8:
        return b"%.9g" % random.uniform(-1e-38, 1e-38)  # f32 subnormal region
    return b"%d" % random.randrange(-10**12, 10**12)

fails = 0
N = 4000
for it in range(N):
    lines = [gen_token() for _ in range(random.randrange(0, 4))]
    data = b"\n".join(lines)
    if random.random() < 0.8:
        data += b"\n"
    a = run(C, data)
    b = run(R, data)
    if a != b:
        fails += 1
        print("FAIL input=%r\n   C: %r\n   R: %r" % (data, a, b))
        if fails > 15:
            break
print("fuzz done: %d failures" % fails)
sys.exit(1 if fails else 0)
