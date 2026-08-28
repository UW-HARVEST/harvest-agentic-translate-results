#!/usr/bin/env python3

import ctypes
import os
import pathlib
import tempfile


class AlertData(ctypes.Structure):
    _fields_ = [
        ("rule", ctypes.c_uint),
        ("level", ctypes.c_uint),
        ("alertid", ctypes.c_char_p),
        ("date", ctypes.c_char_p),
        ("location", ctypes.c_char_p),
        ("comment", ctypes.c_char_p),
        ("group", ctypes.c_char_p),
        ("srcip", ctypes.c_char_p),
        ("srcport", ctypes.c_int),
        ("dstip", ctypes.c_char_p),
        ("dstport", ctypes.c_int),
        ("user", ctypes.c_char_p),
        ("filename", ctypes.c_char_p),
    ]


class Tm(ctypes.Structure):
    _fields_ = [
        ("tm_sec", ctypes.c_int),
        ("tm_min", ctypes.c_int),
        ("tm_hour", ctypes.c_int),
        ("tm_mday", ctypes.c_int),
        ("tm_mon", ctypes.c_int),
        ("tm_year", ctypes.c_int),
        ("tm_wday", ctypes.c_int),
        ("tm_yday", ctypes.c_int),
        ("tm_isdst", ctypes.c_int),
        ("tm_gmtoff", ctypes.c_long),
        ("tm_zone", ctypes.c_void_p),
    ]


class FileQueue(ctypes.Structure):
    _fields_ = [
        ("last_change", ctypes.c_long),
        ("year", ctypes.c_int),
        ("day", ctypes.c_int),
        ("flags", ctypes.c_int),
        ("mon", ctypes.c_char * 4),
        ("file_name", ctypes.c_char * 257),
        ("fp", ctypes.c_void_p),
        ("f_status", ctypes.c_ubyte * 144),
    ]


FIELDS = [name for name, _ in AlertData._fields_]


def configure(path):
    lib = ctypes.CDLL(path, mode=os.RTLD_LOCAL)
    lib.GetAlertData.argtypes = [ctypes.c_int, ctypes.c_void_p]
    lib.GetAlertData.restype = ctypes.POINTER(AlertData)
    lib.FreeAlertData.argtypes = [ctypes.POINTER(AlertData)]
    lib.FreeAlertData.restype = None
    lib.Init_FileQueue.argtypes = [
        ctypes.POINTER(FileQueue),
        ctypes.POINTER(Tm),
        ctypes.c_int,
    ]
    lib.Init_FileQueue.restype = ctypes.c_int
    lib.driver.argtypes = [
        ctypes.c_int,
        ctypes.c_int,
        ctypes.c_int,
        ctypes.c_uint,
        ctypes.c_int,
    ]
    lib.driver.restype = ctypes.POINTER(AlertData)
    lib.os_strdup.argtypes = [ctypes.c_char_p]
    lib.os_strdup.restype = ctypes.c_void_p
    lib.merror.argtypes = [
        ctypes.c_char_p,
        ctypes.c_char_p,
        ctypes.c_int,
        ctypes.c_char_p,
    ]
    lib.merror.restype = None
    return lib


libc = ctypes.CDLL(None)
libc.fopen.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
libc.fopen.restype = ctypes.c_void_p
libc.fclose.argtypes = [ctypes.c_void_p]
libc.fclose.restype = ctypes.c_int
libc.ftell.argtypes = [ctypes.c_void_p]
libc.ftell.restype = ctypes.c_long
libc.free.argtypes = [ctypes.c_void_p]
libc.free.restype = None


def snapshot(pointer):
    if not pointer:
        return None
    value = pointer.contents
    return tuple(getattr(value, field) for field in FIELDS)


def parse_sequence(lib, content, flag=0, calls=3):
    with tempfile.NamedTemporaryFile() as stream:
        stream.write(content)
        stream.flush()
        fp = libc.fopen(os.fsencode(stream.name), b"r")
        assert fp
        results = []
        try:
            for _ in range(calls):
                pointer = lib.GetAlertData(flag, fp)
                results.append((snapshot(pointer), libc.ftell(fp)))
                if pointer:
                    lib.FreeAlertData(pointer)
        finally:
            libc.fclose(fp)
        return results


def capture_stderr(callback):
    saved = os.dup(2)
    with tempfile.TemporaryFile() as output:
        try:
            os.dup2(output.fileno(), 2)
            callback()
            libc.fflush(None)
        finally:
            os.dup2(saved, 2)
            os.close(saved)
        output.seek(0)
        return output.read()


def queue_snapshot(queue):
    return (
        queue.last_change,
        queue.year,
        queue.day,
        queue.flags,
        bytes(queue.mon),
        bytes(queue.file_name),
        bool(queue.fp),
        bytes(queue.f_status),
    )


def run():
    root = pathlib.Path(__file__).resolve().parents[2]
    ref_path = pathlib.Path(os.environ["C_REFERENCE_SO"])
    rust_path = root / "translation/target/release/libdriver.so"
    c_lib = configure(ref_path)
    rust_lib = configure(rust_path)

    assert ctypes.sizeof(AlertData) == 96
    assert ctypes.sizeof(Tm) == 56
    assert ctypes.sizeof(FileQueue) == 440
    assert FileQueue.fp.offset == 288
    assert FileQueue.f_status.offset == 296

    ordinary = (
        b"ignored before an alert\n"
        b"** Alert 123.45: mail - authentication,syslog\n"
        b"2026 Aug 27 12:34:56 host->/var/log/auth.log\n"
        b"Rule: 5710 (level 5) -> 'Attempted login'\n"
        b"Src IP: 192.0.2.1\n"
        b"Src Port: 12345\n"
        b"Dst IP: 198.51.100.2\n"
        b"Dst Port: 22\n"
        b"User: alice\n"
        b"free-form log line\n"
        b"** Alert second: mail - syscheck\n"
        b"2026 Aug 27 12:35:57 host->syscheck\n"
        b"Rule: 550 (level 7) -> 'Integrity changed'\n"
        b"Integrity checksum changed for: '/etc/passwd'\n"
    )
    syscheck = (
        b"** Alert f: mail - syscheck,rootcheck\n"
        b"2026 Aug 27 01:02:03 host->syscheck\n"
        b"Rule: -5 (level -2) -> 'Odd values'\n"
        b"Integrity checksum changed for: '/tmp/file'\n"
    )
    mail_filter = (
        b"** Alert skip: nomail - first\n"
        b"2026 Aug 27 01:02:03 ignored\n"
        b"Rule: 1 (level 1) -> 'Ignored'\n"
        b"** Alert keep: mail - second\n"
        b"2026 Aug 27 04:05:06 accepted\n"
        b"Rule: 2 (level 3) -> 'Accepted'\n"
    )
    malformed = (
        b"** Alert bad: mail - group\n"
        b"2026 Aug 27 01:02:03 location\n"
        b"Rule: 1 (level 2) -> no quote"
    )
    ignored = b"plain text\n** Alert without-colon\nmore text\n"

    cases = [
        ("ordinary records and cursor", ordinary, 0, 3),
        ("syscheck filename and signed atoi", syscheck, 0, 2),
        ("mail filtering", mail_filter, 1, 2),
        ("malformed final rule", malformed, 0, 1),
        ("ignored input", ignored, 0, 1),
    ]
    for name, content, flag, calls in cases:
        expected = parse_sequence(c_lib, content, flag, calls)
        actual = parse_sequence(rust_lib, content, flag, calls)
        assert actual == expected, (name, expected, actual)

    with tempfile.TemporaryDirectory() as directory:
        old_cwd = os.getcwd()
        try:
            os.chdir(directory)
            pathlib.Path("alerts.log").write_bytes(syscheck)
            c_result = c_lib.driver(27, 7, 126, 0, 4)
            c_value = snapshot(c_result)
            c_lib.FreeAlertData(c_result)
            rust_result = rust_lib.driver(27, 7, 126, 0, 4)
            rust_value = snapshot(rust_result)
            rust_lib.FreeAlertData(rust_result)
            assert rust_value == c_value

            when = Tm(tm_mday=27, tm_mon=7, tm_year=126)
            queues = []
            for lib in (c_lib, rust_lib):
                fp = libc.fopen(b"alerts.log", b"r")
                queue = FileQueue()
                queue.fp = fp
                result = lib.Init_FileQueue(
                    ctypes.byref(queue), ctypes.byref(when), 0x010 | 0x004
                )
                queues.append((result, queue_snapshot(queue)))
                libc.fclose(queue.fp)
            assert queues[1] == queues[0], queues
        finally:
            os.chdir(old_cwd)

    for value in (b"", b"abc", b"contains %s and \xff"):
        outputs = []
        for lib in (c_lib, rust_lib):
            pointer = lib.os_strdup(value)
            outputs.append(ctypes.string_at(pointer))
            libc.free(pointer)
        assert outputs[1] == outputs[0] == value

    template = b"error for '%s': (%d)-(%s)"
    args = (template, b"a-very-long-file-name", 17, b"reason")
    c_error = capture_stderr(lambda: c_lib.merror(*args))
    rust_error = capture_stderr(lambda: rust_lib.merror(*args))
    assert rust_error == c_error

    print(f"differential cases passed: {len(cases) + 4}")


if __name__ == "__main__":
    run()
