#!/usr/bin/env python3
import ctypes
import json
import os
import sys


class Header(ctypes.Structure):
    _fields_ = [
        ("length", ctypes.c_size_t),
        ("capacity", ctypes.c_size_t),
        ("hash_table", ctypes.c_void_p),
        ("temp", ctypes.c_ssize_t),
    ]


class Arena(ctypes.Structure):
    _fields_ = [
        ("storage", ctypes.c_void_p),
        ("remaining", ctypes.c_size_t),
        ("block", ctypes.c_ubyte),
        ("mode", ctypes.c_ubyte),
    ]


class HashIndex(ctypes.Structure):
    _fields_ = [
        ("temp_key", ctypes.c_void_p),
        ("slot_count", ctypes.c_size_t),
        ("used_count", ctypes.c_size_t),
        ("used_count_threshold", ctypes.c_size_t),
        ("used_count_shrink_threshold", ctypes.c_size_t),
        ("tombstone_count", ctypes.c_size_t),
        ("tombstone_count_threshold", ctypes.c_size_t),
        ("seed", ctypes.c_size_t),
        ("slot_count_log2", ctypes.c_size_t),
        ("string", Arena),
        ("storage", ctypes.c_void_p),
    ]


class BinaryEntry(ctypes.Structure):
    _fields_ = [("key", ctypes.c_int), ("value", ctypes.c_int)]


class StringEntry(ctypes.Structure):
    _fields_ = [("key", ctypes.c_char_p), ("value", ctypes.c_int)]


def configure(lib):
    size = ctypes.c_size_t
    void = ctypes.c_void_p
    integer = ctypes.c_int
    ssize = ctypes.c_ssize_t

    lib.stbds_arrgrowf.argtypes = [void, size, size, size]
    lib.stbds_arrgrowf.restype = void
    lib.stbds_arrfreef.argtypes = [void]
    lib.stbds_rand_seed.argtypes = [size]
    lib.stbds_hmput_default.argtypes = [void, size]
    lib.stbds_hmput_default.restype = void
    lib.stbds_hmput_key.argtypes = [void, size, void, size, integer]
    lib.stbds_hmput_key.restype = void
    lib.stbds_hmget_key.argtypes = [void, size, void, size, integer]
    lib.stbds_hmget_key.restype = void
    lib.stbds_hmget_key_ts.argtypes = [
        void,
        size,
        void,
        size,
        ctypes.POINTER(ssize),
        integer,
    ]
    lib.stbds_hmget_key_ts.restype = void
    lib.stbds_hmdel_key.argtypes = [void, size, void, size, size, integer]
    lib.stbds_hmdel_key.restype = void
    lib.stbds_hmfree_func.argtypes = [void, size]
    lib.stbds_shmode_func.argtypes = [size, integer]
    lib.stbds_shmode_func.restype = void
    lib.stbds_stralloc.argtypes = [ctypes.POINTER(Arena), ctypes.c_char_p]
    lib.stbds_stralloc.restype = ctypes.c_char_p
    lib.stbds_strreset.argtypes = [ctypes.POINTER(Arena)]
    lib.helxo.argtypes = [ctypes.c_char]


def header_for(array):
    return Header.from_address(array - ctypes.sizeof(Header))


def map_header(visible, element_size):
    return header_for(visible - element_size)


def table_state(header):
    if not header.hash_table:
        return None
    table = HashIndex.from_address(header.hash_table)
    return [
        table.slot_count,
        table.used_count,
        table.used_count_threshold,
        table.used_count_shrink_threshold,
        table.tombstone_count,
        table.tombstone_count_threshold,
        table.seed,
        table.slot_count_log2,
        table.string.remaining,
        table.string.block,
        table.string.mode,
    ]


def exercise_arrays(lib):
    pointer = None
    trace = []
    expected = []
    for add, minimum, append in [
        (0, 1, []),
        (1, 0, [11]),
        (3, 0, [22, 33, 44]),
        (0, 20, []),
        (17, 0, list(range(100, 117))),
    ]:
        pointer = lib.stbds_arrgrowf(
            pointer, ctypes.sizeof(ctypes.c_int), add, minimum
        )
        header = header_for(pointer)
        for value in append:
            ctypes.c_int.from_address(
                pointer + header.length * ctypes.sizeof(ctypes.c_int)
            ).value = value
            header.length += 1
            expected.append(value)
        values = [
            ctypes.c_int.from_address(
                pointer + index * ctypes.sizeof(ctypes.c_int)
            ).value
            for index in range(header.length)
        ]
        assert values == expected
        trace.append([header.length, header.capacity, values])
    lib.stbds_arrfreef(pointer)
    return trace


def exercise_binary_map(lib):
    element_size = ctypes.sizeof(BinaryEntry)
    lib.stbds_rand_seed(0x1020304050607080)
    visible = lib.stbds_hmput_default(None, element_size)
    default = BinaryEntry.from_address(visible - element_size)
    default.key = -1
    default.value = -700

    for key_value in list(range(96)) + [3, 17, 63, 3]:
        key = ctypes.c_int(key_value)
        visible = lib.stbds_hmput_key(
            visible, element_size, ctypes.byref(key), ctypes.sizeof(key), 0
        )
        header = map_header(visible, element_size)
        entry = BinaryEntry.from_address(visible + header.temp * element_size)
        entry.key = key_value
        entry.value = key_value * 13 + 5

    checkpoints = []
    for key_value in [-5, 0, 3, 17, 63, 95, 1000]:
        key = ctypes.c_int(key_value)
        visible = lib.stbds_hmget_key(
            visible, element_size, ctypes.byref(key), ctypes.sizeof(key), 0
        )
        header = map_header(visible, element_size)
        entry = BinaryEntry.from_address(visible + header.temp * element_size)
        checkpoints.append([key_value, header.temp, entry.key, entry.value])

    ts_results = []
    for key_value in [8, 1001]:
        key = ctypes.c_int(key_value)
        temporary = ctypes.c_ssize_t(999)
        visible = lib.stbds_hmget_key_ts(
            visible,
            element_size,
            ctypes.byref(key),
            ctypes.sizeof(key),
            ctypes.byref(temporary),
            0,
        )
        ts_results.append([key_value, temporary.value])

    delete_results = []
    for key_value in list(range(0, 80, 2)) + [1000, 3, 17]:
        key = ctypes.c_int(key_value)
        visible = lib.stbds_hmdel_key(
            visible,
            element_size,
            ctypes.byref(key),
            ctypes.sizeof(key),
            0,
            0,
        )
        header = map_header(visible, element_size)
        delete_results.append(
            [key_value, header.temp, header.length - 1, table_state(header)]
        )

    header = map_header(visible, element_size)
    entries = [
        [entry.key, entry.value]
        for entry in (
            BinaryEntry.from_address(visible + index * element_size)
            for index in range(header.length - 1)
        )
    ]
    result = {
        "checkpoints": checkpoints,
        "thread_safe": ts_results,
        "deletes": delete_results,
        "entries": entries,
        "header": [header.length, header.capacity, header.temp],
        "table": table_state(header),
        "default": [default.key, default.value],
    }
    lib.stbds_hmfree_func(visible - element_size, element_size)
    return result


def exercise_string_map(lib, storage_mode):
    element_size = ctypes.sizeof(StringEntry)
    lib.stbds_rand_seed(0x8877665544332211)
    visible = (
        None
        if storage_mode == 1
        else lib.stbds_shmode_func(element_size, storage_mode)
    )
    held_keys = []
    names = [f"key_{index:03d}".encode() for index in range(70)]
    for index, name in enumerate(names):
        key = ctypes.create_string_buffer(name)
        held_keys.append(key)
        visible = lib.stbds_hmput_key(
            visible,
            element_size,
            ctypes.cast(key, ctypes.c_void_p),
            ctypes.sizeof(ctypes.c_void_p),
            1,
        )
        header = map_header(visible, element_size)
        StringEntry.from_address(
            visible + header.temp * element_size
        ).value = index * 7

    duplicate = ctypes.create_string_buffer(b"key_013")
    held_keys.append(duplicate)
    visible = lib.stbds_hmput_key(
        visible,
        element_size,
        ctypes.cast(duplicate, ctypes.c_void_p),
        ctypes.sizeof(ctypes.c_void_p),
        1,
    )
    header = map_header(visible, element_size)
    StringEntry.from_address(visible + header.temp * element_size).value = 913

    gets = []
    for name in [b"key_000", b"key_013", b"key_069", b"absent"]:
        key = ctypes.create_string_buffer(name)
        visible = lib.stbds_hmget_key(
            visible,
            element_size,
            ctypes.cast(key, ctypes.c_void_p),
            ctypes.sizeof(ctypes.c_void_p),
            1,
        )
        header = map_header(visible, element_size)
        entry = StringEntry.from_address(visible + header.temp * element_size)
        gets.append(
            [
                name.decode(),
                header.temp,
                None if not entry.key else entry.key.decode(),
                entry.value,
            ]
        )

    deletes = []
    for index in list(range(0, 55, 3)) + [13, 69, 200]:
        name = f"key_{index:03d}".encode()
        key = ctypes.create_string_buffer(name)
        visible = lib.stbds_hmdel_key(
            visible,
            element_size,
            ctypes.cast(key, ctypes.c_void_p),
            ctypes.sizeof(ctypes.c_void_p),
            0,
            1,
        )
        header = map_header(visible, element_size)
        deletes.append([name.decode(), header.temp, header.length - 1])

    header = map_header(visible, element_size)
    entries = []
    for index in range(header.length - 1):
        entry = StringEntry.from_address(visible + index * element_size)
        entries.append([entry.key.decode(), entry.value])
    result = {
        "gets": gets,
        "deletes": deletes,
        "entries": entries,
        "header": [header.length, header.capacity, header.temp],
        "table": table_state(header),
    }
    lib.stbds_hmfree_func(visible - element_size, element_size)
    return result


def exercise_arena(lib):
    arena = Arena()
    values = [
        b"a",
        b"medium-string",
        b"x" * 500,
        b"y" * 900,
        b"z" * 600_000,
        b"tail",
    ]
    trace = []
    for value in values:
        result = lib.stbds_stralloc(ctypes.byref(arena), value)
        assert result == value
        blocks = 0
        block = arena.storage
        while block:
            blocks += 1
            block = ctypes.c_void_p.from_address(block).value
        trace.append([len(value), arena.remaining, arena.block, blocks])
    lib.stbds_strreset(ctypes.byref(arena))
    trace.append(
        [
            arena.storage or 0,
            arena.remaining,
            arena.block,
            arena.mode,
        ]
    )
    return trace


def capture_helxo(lib):
    read_fd, write_fd = os.pipe()
    saved_stdout = os.dup(1)
    try:
        os.dup2(write_fd, 1)
        os.close(write_fd)
        lib.stbds_rand_seed(0x31415926)
        lib.helxo(b"q")
        ctypes.CDLL(None).fflush(None)
        os.dup2(saved_stdout, 1)
        chunks = []
        while True:
            chunk = os.read(read_fd, 4096)
            if not chunk:
                break
            chunks.append(chunk)
        return b"".join(chunks).decode()
    finally:
        try:
            os.dup2(saved_stdout, 1)
        except OSError:
            pass
        os.close(saved_stdout)
        os.close(read_fd)


def main():
    lib = ctypes.CDLL(sys.argv[1])
    configure(lib)
    result = {
        "helxo": capture_helxo(lib),
        "arrays": exercise_arrays(lib),
        "binary": exercise_binary_map(lib),
        "string_default": exercise_string_map(lib, 1),
        "string_strdup": exercise_string_map(lib, 2),
        "string_arena": exercise_string_map(lib, 3),
        "arena": exercise_arena(lib),
    }
    json.dump(result, sys.stdout, sort_keys=True, separators=(",", ":"))
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
