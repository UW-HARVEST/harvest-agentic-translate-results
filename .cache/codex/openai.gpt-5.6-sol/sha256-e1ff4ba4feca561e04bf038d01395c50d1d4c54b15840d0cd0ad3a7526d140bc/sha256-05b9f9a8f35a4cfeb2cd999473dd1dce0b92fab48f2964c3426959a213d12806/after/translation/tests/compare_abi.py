#!/usr/bin/env python3

import ctypes
import sys


class StringBuffer(ctypes.Structure):
    _fields_ = [
        ("data", ctypes.c_void_p),
        ("capacity", ctypes.c_int),
        ("length", ctypes.c_int),
    ]


library = ctypes.CDLL(sys.argv[1])
libc = ctypes.CDLL(None)

library.create_buffer.argtypes = [ctypes.c_int]
library.create_buffer.restype = ctypes.POINTER(StringBuffer)
library.append_to_buffer.argtypes = [
    ctypes.POINTER(StringBuffer),
    ctypes.c_char_p,
]
library.append_to_buffer.restype = ctypes.c_int
library.destroy_buffer.argtypes = [ctypes.POINTER(StringBuffer)]
library.destroy_buffer.restype = None
library.get_operation_name.argtypes = [ctypes.c_int]
library.get_operation_name.restype = ctypes.c_char_p
library.perform_operation.argtypes = [
    ctypes.c_int,
    ctypes.c_int,
    ctypes.c_char_p,
]
library.perform_operation.restype = ctypes.c_int
library.buffapp.argtypes = [
    ctypes.c_int,
    ctypes.c_int,
    ctypes.c_int,
    ctypes.c_int,
]
library.buffapp.restype = ctypes.c_int

for operation_code in (-5, -1, 0, 1, 2, 3, 4, 9):
    name = library.get_operation_name(operation_code).decode("ascii")
    print(f"name {operation_code} {name}")

operation_cases = (
    (7, 5, b"add"),
    (7, 5, b"subtract"),
    (7, 5, b"multiply"),
    (7, 5, b"divide"),
    (-7, 5, b"divide"),
    (7, -5, b"divide"),
    (7, 0, b"divide"),
    (7, 5, b"unknown"),
    (7, 5, b"ADD"),
)
for first, second, operation in operation_cases:
    result = library.perform_operation(first, second, operation)
    print(f"operation {first} {second} {operation.decode('ascii')} {result}")

buffer = library.create_buffer(4)
print(
    f"buffer initial {buffer.contents.capacity} "
    f"{buffer.contents.length} {ctypes.string_at(buffer.contents.data)!r}"
)
for value in (b"abc", b"!", b"0123456789", b""):
    result = library.append_to_buffer(buffer, value)
    print(
        f"buffer append {value!r} {result} {buffer.contents.capacity} "
        f"{buffer.contents.length} {ctypes.string_at(buffer.contents.data)!r}"
    )
library.destroy_buffer(buffer)
library.destroy_buffer(None)

sys.stdout.flush()
for parameters in (
    (0, 5, 1, 3),
    (2, 4, 3, 2),
    (-1, 4, -4, 2),
    (3, 0, 0, 7),
    (1, 2, 3, 4),
):
    result = library.buffapp(*parameters)
    libc.fflush(None)
    print(f"buffapp {parameters!r} {result}")
    sys.stdout.flush()
