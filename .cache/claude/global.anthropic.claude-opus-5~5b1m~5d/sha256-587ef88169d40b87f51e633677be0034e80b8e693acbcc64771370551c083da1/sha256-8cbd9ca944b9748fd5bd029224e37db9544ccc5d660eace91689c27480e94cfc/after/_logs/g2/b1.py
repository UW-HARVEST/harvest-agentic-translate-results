import ctypes, sys
which = sys.argv[1]
path = {"c":"$HARVEST_WORKDIR/c_src/build/libsodium.so",
        "r":"$HARVEST_WORKDIR/translation/target/release/liblibsodium.so"}[which]
L = ctypes.CDLL(path)
L.sodium_init()
f = L._sodium_argon2i_hash_encoded
f.argtypes=[ctypes.c_uint32]*3+[ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t, ctypes.c_size_t, ctypes.c_char_p, ctypes.c_size_t]
pw=b"password"; salt=bytes(range(16))
for el in [0,1,8,16,32,64,92,93,128]:
    buf = ctypes.create_string_buffer(el+8)
    sys.stdout.write("encodedlen=%d ... " % el); sys.stdout.flush()
    rc = f(3,8,1,pw,len(pw),salt,16,32,buf,el)
    print("rc=%d" % rc)
