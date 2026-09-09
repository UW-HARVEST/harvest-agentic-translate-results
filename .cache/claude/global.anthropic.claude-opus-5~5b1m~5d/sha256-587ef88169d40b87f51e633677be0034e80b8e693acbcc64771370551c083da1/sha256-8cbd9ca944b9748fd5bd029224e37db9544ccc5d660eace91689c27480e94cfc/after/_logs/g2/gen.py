import ctypes, sys
L = ctypes.CDLL("$HARVEST_WORKDIR/c_src/build/libsodium.so")
assert L.sodium_init() in (0,1)
# raw argon2 hash_raw is internal; use crypto_pwhash for outlen, and bin2base64 for encoding
L.sodium_bin2base64.restype = ctypes.c_char_p
def b64(b):
    n = len(b)
    out = ctypes.create_string_buffer(n*2+16)
    L.sodium_bin2base64(out, len(out), b, ctypes.c_size_t(n), 3)
    return out.value.decode()

L.crypto_pwhash.argtypes=[ctypes.c_char_p, ctypes.c_ulonglong, ctypes.c_char_p, ctypes.c_ulonglong, ctypes.c_char_p, ctypes.c_ulonglong, ctypes.c_size_t, ctypes.c_int]
def pwhash(outlen, pw, salt, ops, mem, alg):
    out = ctypes.create_string_buffer(outlen)
    r = L.crypto_pwhash(out, outlen, pw, len(pw), salt, ops, mem, alg)
    assert r==0, r
    return out.raw[:outlen]

salt16 = bytes(range(16))
pw = b"password"
# argon2id v=19 m=8 t=1 p=1, 32-byte hash
h = pwhash(32, pw, salt16, 1, 8192, 2)
print('ARGON2ID_VEC = "$argon2id$v=19$m=8,t=1,p=1$%s$%s"' % (b64(salt16), b64(h)))
h = pwhash(32, pw, salt16, 3, 8192, 1)
print('ARGON2I_VEC  = "$argon2i$v=19$m=8,t=3,p=1$%s$%s"' % (b64(salt16), b64(h)))
# 64-byte hash argon2id, m=8,t=1,p=1
h = pwhash(64, pw, salt16, 1, 8192, 2)
print('ARGON2ID_VEC64 = "$argon2id$v=19$m=8,t=1,p=1$%s$%s"' % (b64(salt16), b64(h)))
# m=9 (memlimit 9216)
h = pwhash(32, pw, salt16, 1, 9216, 2)
print('ARGON2ID_M9 = "$argon2id$v=19$m=9,t=1,p=1$%s$%s"' % (b64(salt16), b64(h)))
# m=16 t=1 p=1
h = pwhash(32, pw, salt16, 1, 16384, 2)
print('ARGON2ID_M16 = "$argon2id$v=19$m=16,t=1,p=1$%s$%s"' % (b64(salt16), b64(h)))
# str vectors
out = ctypes.create_string_buffer(128)
L.crypto_pwhash_str.argtypes=[ctypes.c_char_p, ctypes.c_char_p, ctypes.c_ulonglong, ctypes.c_ulonglong, ctypes.c_size_t]
r = L.crypto_pwhash_str(out, pw, len(pw), 1, 8192); assert r==0
print("STR sample:", out.value.decode(), len(out.value))
# scrypt str
o2 = ctypes.create_string_buffer(102)
L.crypto_pwhash_scryptsalsa208sha256_str.argtypes=[ctypes.c_char_p, ctypes.c_char_p, ctypes.c_ulonglong, ctypes.c_ulonglong, ctypes.c_size_t]
r = L.crypto_pwhash_scryptsalsa208sha256_str(o2, pw, len(pw), 32768, 16777216); assert r==0
print("SCRYPT_STR =", repr(o2.value.decode()), len(o2.value))
