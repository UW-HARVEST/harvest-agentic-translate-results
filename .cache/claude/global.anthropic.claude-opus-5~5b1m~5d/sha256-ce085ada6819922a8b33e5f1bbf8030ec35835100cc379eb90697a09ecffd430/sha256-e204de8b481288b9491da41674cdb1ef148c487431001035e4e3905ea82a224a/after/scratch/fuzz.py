import random, subprocess, sys, os
W="$HARVEST_WORKDIR"
C=W+"/c_src/build/driver"; R=W+"/translation/target/release/driver"

def run(p, data):
    pr = subprocess.run([p], input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return pr.stdout, pr.stderr, pr.returncode

INTS = ["0","1","-1","2","3","4","5","6","7","-2147483648","2147483647","2147483648",
        "4294967296","99999999999999999999","256","257","255","100","101","-5","+3",
        "0007","abc","x","","0x10","-","+","1e5","999999999999"]
WS = [" ","\n","\t","  ","\n\n","\r\n"]

def gen(rnd):
    toks=[]
    k = rnd.randint(0,60)
    for _ in range(k):
        if rnd.random()<0.75:
            toks.append(str(rnd.choice([0,1,2,3,4,5,6,7,-1,256,257,100,101,
                                        rnd.randint(-300,300)])))
        else:
            toks.append(rnd.choice(INTS))
    s=""
    for t in toks:
        s += t + rnd.choice(WS)
    return s.encode()

def structured(rnd):
    op = rnd.choice([0,1,2,3,4,5,6,7,-1,rnd.randint(-3,9)])
    cnt = rnd.choice([1,2,3,rnd.randint(1,6)])
    parts=[str(op), str(cnt)]
    for _ in range(cnt):
        L = rnd.choice([0,1,2,3,128,129,256,rnd.randint(0,10)])
        parts.append(str(L))
        parts += [str(rnd.choice([0,1,255,256,-1,rnd.randint(-5,300)])) for _ in range(L)]
    if op in (3,5) and rnd.random()<0.9:
        parts.append(str(rnd.choice([0,1,-1,2,3,256,-256,2147483647,-2147483648,rnd.randint(-300,300)])))
    return (" ".join(parts)+"\n").encode()

rnd=random.Random(int(sys.argv[1]) if len(sys.argv)>1 else 0)
N=int(sys.argv[2]) if len(sys.argv)>2 else 3000
bad=0
for i in range(N):
    data = gen(rnd) if i%2 else structured(rnd)
    a=run(C,data); b=run(R,data)
    if a!=b:
        bad+=1
        print("MISMATCH", repr(data[:300]))
        print(" C:", a[0][:200], a[1][:200], a[2])
        print(" R:", b[0][:200], b[1][:200], b[2])
        if bad>=8: break
print("done, mismatches:", bad, "of", N)
