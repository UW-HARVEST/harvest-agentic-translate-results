#!/usr/bin/env python3
"""Generate Rust implementation of for_gen functions, mirroring c_src/gen.pl.

The C file uses `*(uint32_t *)ptr` reads/writes which assume little-endian.
We mirror this with u32::from_le_bytes / to_le_bytes.

The pack functions:  fn pack{bits}_{block}(base, in: &[u32], out: &mut [u8]) -> u32
The unpack functions: fn unpack{bits}_{block}(base, in: &[u8], out: &mut [u32]) -> u32
The linsearch funcs:  fn linsearch{bits}_{block}(base, in: &[u8], value, found: &mut i32) -> u32

For block in {32, 16, 8} (fixed) and "x" (variable length).
"""

import sys

OUT = []

def emit(line=""):
    OUT.append(line)


def read_u32(buf, off):
    """Generate a Rust expression to read u32 from buf[off..off+4]."""
    return f"u32::from_le_bytes([{buf}[{off}], {buf}[{off}+1], {buf}[{off}+2], {buf}[{off}+3]])"


def write_u32_stmts(buf, off, val_expr, indent="    "):
    """Generate a Rust statement to write u32 to buf[off..off+4]."""
    return f"{indent}{{ let __b = ({val_expr}).to_le_bytes(); {buf}[{off}] = __b[0]; {buf}[{off}+1] = __b[1]; {buf}[{off}+2] = __b[2]; {buf}[{off}+3] = __b[3]; }}"


def write_u32_partial_stmts(buf, off, val_expr, length_expr, indent="    "):
    """Generate Rust statement to copy first `length_expr` bytes of u32 val to buf[off..]."""
    return (
        f"{indent}{{ let __b = ({val_expr}).to_le_bytes(); "
        f"let __len = ({length_expr}) as usize; "
        f"for __i in 0..__len {{ {buf}[{off}+__i] = __b[__i]; }} }}"
    )


def gen_pack_impl(fname, bits, block):
    emit(f"pub fn {fname}(base: u32, input: &[u32], output: &mut [u8]) -> u32 {{")
    if bits == 0:
        emit("    let _ = (base, input, output);")
        emit("    0")
        emit("}")
        return
    if bits == 32:
        emit(f"    for i in 0..{block} {{")
        emit(f"        let v = input[i].wrapping_sub(base);")
        emit(f"        let off = i * 4;")
        emit(f"        let b = v.to_le_bytes();")
        emit(f"        output[off] = b[0]; output[off+1] = b[1]; output[off+2] = b[2]; output[off+3] = b[3];")
        emit(f"    }}")
        emit(f"    {block * 4}")
        emit("}")
        return

    bits_per_word = 32
    emit("    let mut tmp: u32 = 0;")
    emit("    let mut out_off: usize = 0;")
    consumed = 0
    inittmp = True
    i = 0
    j = 0
    while True:
        b = 0
        while b < bits_per_word and i < block:
            if b + bits <= bits_per_word:
                if inittmp:
                    emit(f"    tmp = input[{j}].wrapping_sub(base) << {b};")
                    inittmp = False
                else:
                    emit(f"    tmp |= input[{j}].wrapping_sub(base) << {b};")
                b += bits
            else:
                emit(f"    tmp |= input[{j}].wrapping_sub(base) << {b};")
                emit(write_u32_stmts("output", "out_off", "tmp"))
                emit(f"    out_off += 4;")
                consumed += bits_per_word // 8
                d = (b + bits) - 32
                emit(f"    tmp = input[{j}].wrapping_sub(base) >> ({bits} - {d});")
                b = d
            j += 1
            i += 1

        if i < block:
            emit(write_u32_stmts("output", "out_off", "tmp"))
            emit(f"    out_off += 4;")
            inittmp = True
            consumed += bits_per_word // 8
        else:
            remaining_bits = bits_per_word - b
            length_bytes = (bits_per_word // 8) - (remaining_bits // 8)
            consumed += length_bytes
            # memcpy(out, &tmp, length)
            emit(write_u32_partial_stmts("output", "out_off", "tmp", str(length_bytes)))
            emit(f"    {consumed}")
            break
    emit("}")


def gen_unpack_impl(fname, bits, block):
    emit(f"pub fn {fname}(base: u32, input: &[u8], output: &mut [u32]) -> u32 {{")
    if bits == 0:
        emit(f"    let _ = input;")
        emit(f"    for k in 0..{block} {{ output[k] = base; }}")
        emit("    0")
        emit("}")
        return
    if bits == 32:
        emit(f"    for i in 0..{block} {{")
        emit(f"        let off = i * 4;")
        emit(f"        let v = u32::from_le_bytes([input[off], input[off+1], input[off+2], input[off+3]]);")
        emit(f"        output[i] = base.wrapping_add(v);")
        emit(f"    }}")
        emit(f"    {block * 4}")
        emit("}")
        return

    bits_per_word = 32
    mask = (1 << bits) - 1
    emit("    let mut in32_off: usize = 0;")
    emit("    let mut tmp: u32;")
    consumed = 4
    i = 0
    j = 0
    while True:
        b = 0
        while b < bits_per_word and i < block:
            if b + bits <= bits_per_word:
                emit(f"    output[{j}] = base.wrapping_add(({read_u32('input', 'in32_off')} >> {b}) & {mask});")
                b += bits
            else:
                emit(f"    tmp = {read_u32('input', 'in32_off')} >> {b};")
                emit(f"    in32_off += 4;")
                consumed += bits_per_word // 8
                d = (b + bits) - 32
                emit(f"    tmp |= ({read_u32('input', 'in32_off')} % (1u32 << {d})) << ({bits} - {d});")
                emit(f"    output[{j}] = base.wrapping_add(tmp);")
                b = d
            j += 1
            i += 1

        if i < block:
            emit(f"    in32_off += 4;")
            consumed += bits_per_word // 8
        else:
            remaining_bits = bits_per_word - b
            consumed -= remaining_bits // 8
            emit(f"    {consumed}")
            break
    emit("}")


def gen_packx_impl(fname, bits):
    block = 8
    bits_per_word = 32
    emit(f"pub fn {fname}(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {{")
    if bits == 0:
        emit(f"    let _ = (base, input, output, length);")
        emit("    0")
        emit("}")
        return
    emit("    if length == 0 { return 0; }")
    emit("    let mut tmp: u32 = 0;")
    emit("    let mut out_off: usize = 0;")
    inittmp = True
    i = 0
    j = 0
    bail_emitted = False

    # We will collect all body lines into a list and at the end add the bail label.
    bail_label_idx = None
    while True:
        b = 0
        while b < bits_per_word and i < block:
            if b + bits <= bits_per_word:
                if inittmp:
                    emit(f"    tmp = input[{j}].wrapping_sub(base) << {b};")
                    inittmp = False
                else:
                    emit(f"    tmp |= input[{j}].wrapping_sub(base) << {b};")
                b += bits
            else:
                emit(f"    tmp |= input[{j}].wrapping_sub(base) << {b};")
                emit(write_u32_stmts("output", "out_off", "tmp"))
                emit(f"    out_off += 4;")
                d = (b + bits) - 32
                emit(f"    tmp = input[{j}].wrapping_sub(base) >> ({bits} - {d});")
                b = d
            j += 1
            i += 1
            # if (length == j) goto bail;
            emit(f"    if length == {j} {{")
            # write tmp partial bytes and return
            emit(f"        let mut remaining = ((length * {bits}) + 7) / 8 % 4;")
            emit(f"        if remaining == 0 {{ remaining = 4; }}")
            emit(write_u32_partial_stmts("output", "out_off", "tmp", "remaining", indent="        "))
            emit(f"        return ((length * {bits}) + 7) / 8;")
            emit(f"    }}")

        if i < block:
            emit(write_u32_stmts("output", "out_off", "tmp"))
            emit(f"    out_off += 4;")
            inittmp = True
        else:
            # End reached - we still need a fallthrough bail (length==block already checked but for safety)
            emit(f"    let mut remaining = ((length * {bits}) + 7) / 8 % 4;")
            emit(f"    if remaining == 0 {{ remaining = 4; }}")
            emit(write_u32_partial_stmts("output", "out_off", "tmp", "remaining"))
            emit(f"    ((length * {bits}) + 7) / 8")
            break
    emit("}")


def gen_unpackx_impl(fname, bits):
    block = 8
    bits_per_word = 32
    mask = (1 << bits) - 1
    emit(f"pub fn {fname}(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {{")
    if bits == 0:
        emit(f"    let _ = input;")
        emit(f"    for k in 0..(length as usize) {{ output[k] = base; }}")
        emit("    0")
        emit("}")
        return
    emit("    if length == 0 { return 0; }")
    emit("    let mut in_off: usize = 0;")
    emit("    let mut tmp: u32 = 0;")
    inittmp = True
    i = 0
    j = 0
    while True:
        b = 0
        while b < bits_per_word and i < block:
            if inittmp:
                emit(f"    tmp = {read_u32('input', 'in_off')};")
                inittmp = False

            if b + bits <= bits_per_word:
                emit(f"    output[{j}] = base.wrapping_add((tmp >> {b}) & {mask});")
                b += bits
            else:
                emit(f"    output[{j}] = tmp >> {b};")
                emit(f"    in_off += 4;")
                d = (b + bits) - 32
                emit(f"    tmp = {read_u32('input', 'in_off')};")
                emit(f"    output[{j}] |= (tmp % (1u32 << {d})) << ({bits} - {d});")
                emit(f"    output[{j}] = output[{j}].wrapping_add(base);")
                b = d
            j += 1
            i += 1
            emit(f"    if length == {j} {{ return ((length * {bits}) + 7) / 8; }}")

        if i < block:
            emit(f"    in_off += 4;")
            inittmp = True
        else:
            emit(f"    ((length * {bits}) + 7) / 8")
            break
    emit("}")


def gen_linsearch_impl(fname, bits, block):
    emit(f"pub fn {fname}(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {{")
    if bits == 0:
        emit("    let _ = input;")
        emit("    if base == value { *found = 0; }")
        emit("    0")
        emit("}")
        return
    if bits == 32:
        emit("    let value = value.wrapping_sub(base);")
        emit(f"    for i in 0..{block} {{")
        emit(f"        let off = i * 4;")
        emit(f"        let v = u32::from_le_bytes([input[off], input[off+1], input[off+2], input[off+3]]);")
        emit(f"        if v == value {{ *found = i as i32; return 0; }}")
        emit(f"    }}")
        emit(f"    {block * 4}")
        emit("}")
        return

    bits_per_word = 32
    mask = (1 << bits) - 1
    emit("    let mut in_off: usize = 0;")
    emit("    let mut tmp: u32 = 0;")
    emit("    let mut tmp2: u32;")
    emit("    let value = value.wrapping_sub(base);")
    consumed = 0
    inittmp = True
    i = 0
    j = 0
    while True:
        b = 0
        while b < bits_per_word and i < block:
            if inittmp:
                emit(f"    tmp = {read_u32('input', 'in_off')};")
                consumed += bits_per_word // 8
                inittmp = False

            if b + bits <= bits_per_word:
                emit(f"    if ((tmp >> {b}) & {mask}) == value {{ *found = {j}; return {j}; }}")
                b += bits
            else:
                emit(f"    tmp2 = tmp >> {b};")
                emit(f"    in_off += 4;")
                consumed += bits_per_word // 8
                d = (b + bits) - 32
                emit(f"    tmp = {read_u32('input', 'in_off')};")
                emit(f"    if (tmp2 | (tmp % (1u32 << {d})) << ({bits} - {d})) == value {{ *found = {j}; return {j}; }}")
                b = d
            j += 1
            i += 1

        if i < block:
            emit(f"    in_off += 4;")
            inittmp = True
        else:
            remaining_bits = bits_per_word - b
            consumed -= remaining_bits // 8
            emit(f"    {consumed}")
            break
    emit("}")


def gen_linsearchx_impl(fname, bits):
    block = 8
    bits_per_word = 32
    mask = (1 << bits) - 1
    emit(f"pub fn {fname}(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {{")
    if bits == 0:
        emit("    let _ = input;")
        emit("    if base == value && length > 0 { *found = 0; }")
        emit("    0")
        emit("}")
        return
    emit("    if length == 0 { return 0; }")
    emit("    let mut in_off: usize = 0;")
    emit("    let mut tmp: u32 = 0;")
    emit("    let mut tmp2: u32;")
    emit("    let value = value.wrapping_sub(base);")
    inittmp = True
    i = 0
    j = 0
    while True:
        b = 0
        while b < bits_per_word and i < block:
            if inittmp:
                emit(f"    tmp = {read_u32('input', 'in_off')};")
                inittmp = False

            if b + bits <= bits_per_word:
                emit(f"    if value == ((tmp >> {b}) & {mask}) {{ *found = {j}; return {j}; }}")
                b += bits
            else:
                emit(f"    tmp2 = tmp >> {b};")
                emit(f"    in_off += 4;")
                d = (b + bits) - 32
                emit(f"    tmp = {read_u32('input', 'in_off')};")
                emit(f"    if (tmp2 | (tmp % (1u32 << {d})) << ({bits} - {d})) == value {{ *found = {j}; return {j}; }}")
                b = d
            j += 1
            i += 1
            emit(f"    if length == {j} {{ return ((length * {bits}) + 7) / 8; }}")

        if i < block:
            emit(f"    in_off += 4;")
            inittmp = True
        else:
            emit(f"    ((length * {bits}) + 7) / 8")
            break
    emit("}")


def gen_linsearchx_signature_only_for_zero(fname):
    """linsearch0_x has signature with length param."""
    emit(f"pub fn {fname}(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {{")
    emit("    let _ = input;")
    emit("    if base == value && length > 0 { *found = 0; }")
    emit("    0")
    emit("}")


def main():
    # Prologue functions for the "0-bit" cases (just constant base)
    # pack0_32, pack0_16, pack0_8
    for blk in (32, 16, 8):
        gen_pack_impl(f"pack0_{blk}", 0, blk)
    # unpack0_32, unpack0_16, unpack0_8
    for blk in (32, 16, 8):
        gen_unpack_impl(f"unpack0_{blk}", 0, blk)
    # pack0_x, unpack0_x
    gen_packx_impl("pack0_x", 0)
    gen_unpackx_impl("unpack0_x", 0)
    # linsearch0_32, _16, _8
    for blk in (32, 16, 8):
        gen_linsearch_impl(f"linsearch0_{blk}", 0, blk)
    gen_linsearchx_impl("linsearch0_x", 0)

    # For each block size, generate pack_b_block and unpack_b_block for b=1..32
    for blk in (32, 16, 8):
        for b in range(1, 33):
            gen_pack_impl(f"pack{b}_{blk}", b, blk)
            gen_unpack_impl(f"unpack{b}_{blk}", b, blk)

    # pack_x and unpack_x for b=1..32
    for b in range(1, 33):
        gen_packx_impl(f"pack{b}_x", b)
        gen_unpackx_impl(f"unpack{b}_x", b)

    # linsearch for blocks (32, 16, 8) for b=1..32
    for blk in (32, 16, 8):
        for b in range(1, 33):
            gen_linsearch_impl(f"linsearch{b}_{blk}", b, blk)

    for b in range(1, 33):
        gen_linsearchx_impl(f"linsearch{b}_x", b)

    # Tables
    def array(name, ftype, bsuffix):
        if bsuffix == "x":
            entries = [f"{name}{i}_x" for i in range(33)]
        else:
            entries = [f"{name}{i}_{bsuffix}" for i in range(33)]
        emit(f"pub static for_{name}{bsuffix}: [{ftype}; 33] = [")
        for e in entries:
            emit(f"    {e},")
        emit("];")
        emit("")

    emit("// Function pointer types & tables")
    emit("pub type ForPackFunc = fn(u32, &[u32], &mut [u8]) -> u32;")
    emit("pub type ForUnpackFunc = fn(u32, &[u8], &mut [u32]) -> u32;")
    emit("pub type ForPackXFunc = fn(u32, &[u32], &mut [u8], u32) -> u32;")
    emit("pub type ForUnpackXFunc = fn(u32, &[u8], &mut [u32], u32) -> u32;")
    emit("pub type ForLinsearchFunc = fn(u32, &[u8], u32, &mut i32) -> u32;")
    emit("pub type ForLinsearchXFunc = fn(u32, &[u8], u32, u32, &mut i32) -> u32;")
    emit("")
    emit("#[allow(non_upper_case_globals)]")
    array("pack", "ForPackFunc", "32")
    emit("#[allow(non_upper_case_globals)]")
    array("unpack", "ForUnpackFunc", "32")
    emit("#[allow(non_upper_case_globals)]")
    array("pack", "ForPackFunc", "16")
    emit("#[allow(non_upper_case_globals)]")
    array("unpack", "ForUnpackFunc", "16")
    emit("#[allow(non_upper_case_globals)]")
    array("pack", "ForPackFunc", "8")
    emit("#[allow(non_upper_case_globals)]")
    array("unpack", "ForUnpackFunc", "8")
    emit("#[allow(non_upper_case_globals)]")
    array("pack", "ForPackXFunc", "x")
    emit("#[allow(non_upper_case_globals)]")
    array("unpack", "ForUnpackXFunc", "x")
    emit("#[allow(non_upper_case_globals)]")
    array("linsearch", "ForLinsearchFunc", "32")
    emit("#[allow(non_upper_case_globals)]")
    array("linsearch", "ForLinsearchFunc", "16")
    emit("#[allow(non_upper_case_globals)]")
    array("linsearch", "ForLinsearchFunc", "8")
    emit("#[allow(non_upper_case_globals)]")
    array("linsearch", "ForLinsearchXFunc", "x")

    out_file = sys.argv[1]
    with open(out_file, "w") as f:
        f.write("\n".join(OUT) + "\n")


if __name__ == "__main__":
    main()
