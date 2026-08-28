import random, subprocess
W="$HARVEST_WORKDIR"
C=W+"/c_src/build/driver"; R=W+"/translation/target/release/driver"
def run(p,d):
    pr=subprocess.run([p],input=d,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    return pr.stdout,pr.stderr,pr.returncode
cases=[
 b"\x00\x01\x02", b"6 1\n2 5\x00 7", b"\xff\xfe", b"6\x001\n", b"6 1\n1 \xc3\xa9",
 b"6 1\n" + b"1 " + b"9"*400, b"6 1\n1 " + b"-"+b"9"*400, b"6 " + b"0"*50 + b"1\n1 5",
 b"\n"*5000 + b"6 1\n1 5", b" "*9000 + b"6 1\n1 5",
 b"6 1\n1 5\n" + b"x"*9000, b"3 1\n2 1 2\n" + b"9"*30,
]
rnd=random.Random(7)
for _ in range(600):
    cases.append(bytes(rnd.randrange(256) for _ in range(rnd.randint(0,80))))
for _ in range(600):
    # digits/space/sign heavy random
    alpha=b"0123456789 \n\t-+"
    cases.append(bytes(rnd.choice(alpha) for _ in range(rnd.randint(0,120))))
bad=0
for d in cases:
    a=run(C,d); b=run(R,d)
    if a!=b:
        bad+=1; print("MISMATCH",repr(d[:120])); print(" C:",a); print(" R:",b)
        if bad>6: break
print("done, mismatches:",bad,"of",len(cases))
