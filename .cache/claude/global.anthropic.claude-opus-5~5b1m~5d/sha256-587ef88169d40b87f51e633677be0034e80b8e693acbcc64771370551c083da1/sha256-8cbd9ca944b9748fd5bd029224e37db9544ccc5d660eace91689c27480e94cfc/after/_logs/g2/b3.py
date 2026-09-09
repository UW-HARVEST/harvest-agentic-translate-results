import ctypes, sys
which, dl = sys.argv[1], int(sys.argv[2])
path = {"c":"$HARVEST_WORKDIR/c_src/build/libsodium.so",
        "r":"$HARVEST_WORKDIR/translation/target/release/liblibsodium.so"}[which]
L = ctypes.CDLL(path); L.sodium_init()
class Ctx(ctypes.Structure):
    _fields_=[("out",ctypes.c_void_p),("outlen",ctypes.c_uint32),
              ("pwd",ctypes.c_void_p),("pwdlen",ctypes.c_uint32),
              ("salt",ctypes.c_void_p),("saltlen",ctypes.c_uint32),
              ("secret",ctypes.c_void_p),("secretlen",ctypes.c_uint32),
              ("ad",ctypes.c_void_p),("adlen",ctypes.c_uint32),
              ("t_cost",ctypes.c_uint32),("m_cost",ctypes.c_uint32),
              ("lanes",ctypes.c_uint32),("threads",ctypes.c_uint32),("flags",ctypes.c_uint32)]
o=ctypes.create_string_buffer(33); s=ctypes.create_string_buffer(17)
c=Ctx(); c.out=ctypes.cast(o,ctypes.c_void_p); c.outlen=32
c.salt=ctypes.cast(s,ctypes.c_void_p); c.saltlen=16
c.pwd=None; c.pwdlen=0; c.t_cost=3; c.m_cost=8; c.lanes=1; c.threads=1
f=L._sodium_argon2_encode_string
f.argtypes=[ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(Ctx), ctypes.c_int]
dst=ctypes.create_string_buffer(dl+8)
print("rc=%d" % f(dst, dl, ctypes.byref(c), 2))
