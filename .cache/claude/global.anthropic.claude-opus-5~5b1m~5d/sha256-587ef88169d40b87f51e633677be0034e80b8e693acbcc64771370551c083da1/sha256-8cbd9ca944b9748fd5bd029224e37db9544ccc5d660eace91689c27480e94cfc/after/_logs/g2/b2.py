import ctypes, sys
which, el = sys.argv[1], int(sys.argv[2])
path = {"c":"$HARVEST_WORKDIR/c_src/build/libsodium.so",
        "r":"$HARVEST_WORKDIR/translation/target/release/liblibsodium.so"}[which]
L = ctypes.CDLL(path); L.sodium_init()
f = L._sodium_argon2i_hash_encoded
f.argtypes=[ctypes.c_uint32]*3+[ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t, ctypes.c_size_t, ctypes.c_char_p, ctypes.c_size_t]
buf = ctypes.create_string_buffer(el+8)
rc = f(3,8,1,b"password",8,bytes(range(16)),16,32,buf,el)
print("rc=%d" % rc)
