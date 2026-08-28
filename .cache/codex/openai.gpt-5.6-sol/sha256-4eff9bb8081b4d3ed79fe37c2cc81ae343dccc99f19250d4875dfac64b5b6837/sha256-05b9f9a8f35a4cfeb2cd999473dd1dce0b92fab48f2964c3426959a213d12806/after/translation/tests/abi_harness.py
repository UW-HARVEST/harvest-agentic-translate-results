import ctypes
import sys


lib = ctypes.CDLL(sys.argv[1])

lib.printLine.argtypes = [ctypes.c_char_p]
lib.printLine.restype = None
lib.printIntLine.argtypes = [ctypes.c_int]
lib.printIntLine.restype = None
lib.bad.argtypes = [ctypes.c_int]
lib.bad.restype = None
lib.good.argtypes = [ctypes.c_int]
lib.good.restype = None
lib.driver.argtypes = [ctypes.c_int, ctypes.c_int]
lib.driver.restype = None

lib.printLine(None)
for value in (b"", b"plain text", b"100% %d %s", b"line one\nline two"):
    lib.printLine(value)

for value in (-2147483648, -1, 0, 1, 2147483647):
    lib.printIntLine(value)

for value in (-2147483648, -2, -1, 0, 1, 7, 9):
    lib.bad(value)

for value in (-2147483648, -1, 0, 1, 7, 9, 10, 2147483647):
    lib.good(value)

for values in ((-1, -1), (0, 0), (9, 9), (10, -1), (2147483647, -1)):
    lib.driver(*values)

libc = ctypes.CDLL(None)
libc.fflush.argtypes = [ctypes.c_void_p]
libc.fflush.restype = ctypes.c_int
if libc.fflush(None) != 0:
    raise RuntimeError("fflush failed")
