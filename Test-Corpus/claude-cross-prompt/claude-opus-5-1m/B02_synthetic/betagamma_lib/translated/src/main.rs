// Translation of c_src/src/lib.c (plus a thin main wrapper) to safe Rust.
//
// The C source is a library exposing `betagamma(a, b, c, d)`. Since the
// translation is required to be an executable, we add a small `main` that
// reads four whitespace-separated integers from stdin (mirroring scanf's
// whitespace-skipping behavior across newlines) and prints the result of
// `betagamma` followed by a newline (matching `printf("%d\n", ...)`).

use std::io::{self, Read, Write};

// 32-bit-wraparound int matching C's `int` semantics on common platforms.
type CInt = i32;

#[derive(Clone)]
struct DataBlock {
    id: CInt,
    name: [u8; 32],
    flags: u8,
}

struct MemoryBlock {
    data: Vec<CInt>,
    size: usize,
    // Synthetic "address" used to mirror the C code's pointer comparisons.
    // The original implementation compares pointers from sequential
    // malloc/calloc calls; on glibc those are monotonically increasing for
    // small allocations, so we model that ordering deterministically.
    addr: usize,
    data_addr: usize,
}

fn make_name(s: &str) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let bytes = s.as_bytes();
    let n = bytes.len().min(31);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf
}

#[allow(dead_code)]
fn create_block(id: CInt, name: &str, flags: u8) -> DataBlock {
    DataBlock {
        id,
        name: make_name(name),
        flags,
    }
}

fn allocate_block(count: usize, init_value: CInt, addr: usize, data_addr: usize) -> Option<MemoryBlock> {
    let mut data: Vec<CInt> = Vec::with_capacity(count);
    for i in 0..count {
        // C: mb->data[i] = init_value + i;  (int + size_t implicitly converts
        // size_t to int via assignment; the addition itself is performed in
        // the larger common type but truncates on assignment back to int.)
        let v = (init_value as i64).wrapping_add(i as i64) as CInt;
        data.push(v);
    }
    Some(MemoryBlock {
        data,
        size: count,
        addr,
        data_addr,
    })
}

fn compute_hash(mb1: &MemoryBlock, mb2: &MemoryBlock) -> CInt {
    let mut hash: CInt = 0;

    if mb1.data_addr < mb2.data_addr {
        hash += 100;
    } else if mb1.data_addr > mb2.data_addr {
        hash += 200;
    }

    if mb1.addr < mb2.addr {
        hash += 10;
    } else if mb1.addr > mb2.addr {
        hash += 20;
    }

    hash
}

fn betagamma(param1: CInt, param2: CInt, param3: CInt, param4: CInt) -> CInt {
    let mut result: CInt = 0;

    let blocks: [DataBlock; 3] = [
        DataBlock {
            id: 1,
            name: make_name("Block_Alpha"),
            flags: 0b1010_1010,
        },
        DataBlock {
            id: 2,
            name: make_name("Block_Beta"),
            flags: 0b1100_1100,
        },
        DataBlock {
            id: 3,
            name: make_name("Block_Gamma"),
            flags: 0b1111_0000,
        },
    ];

    for current in blocks.iter() {
        // Mirror the unused `temp_name` copy from the C source.
        let mut temp_name = [0u8; 32];
        temp_name.copy_from_slice(&current.name);
        let _ = temp_name;

        let mut flag_contribution: CInt = 0;
        if current.flags & 0b0000_1111 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param1);
        }
        if current.flags & 0b1111_0000 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param2);
        }
        if current.flags & 0b1010_1010 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param3);
        }
        if current.flags & 0b0101_0101 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param4);
        }

        result = result.wrapping_add(flag_contribution.wrapping_mul(current.id));
    }

    // C: size_t block_size = (param1 % 10) + 5;
    // Note: param1 is signed. In C, % on a negative `int` may yield a negative
    // remainder, which then converts to size_t (a very large unsigned value).
    // Reproduce that exact arithmetic: do the math in signed, then cast to
    // size_t (usize) like C does.
    let rem = param1 % 10;
    let signed_block_size = (rem as i64) + 5;
    let block_size: usize = signed_block_size as usize;

    // Model sequential allocator addresses. The exact numbers don't matter
    // as long as the ordering matches what glibc gives us in practice:
    // mem1 is allocated before mem2, and mem1->data is allocated before
    // mem2->data, so both pointer comparisons place mem1 below mem2.
    let mem1_addr = 0x1000;
    let mem1_data_addr = 0x2000;
    let mem2_addr = 0x3000;
    let mem2_data_addr = 0x4000;

    let mem1 = allocate_block(block_size, param1, mem1_addr, mem1_data_addr);
    let mem2 = allocate_block(block_size, param2, mem2_addr, mem2_data_addr);

    let (mem1, mem2) = match (mem1, mem2) {
        (Some(a), Some(b)) => (a, b),
        _ => return -1,
    };

    let hash = compute_hash(&mem1, &mem2);
    result = result.wrapping_add(hash);

    let mut sum1: CInt = 0;
    let mut sum2: CInt = 0;
    for i in 0..mem1.size {
        sum1 = sum1.wrapping_add(mem1.data[i]);
    }
    for i in 0..mem2.size {
        sum2 = sum2.wrapping_add(mem2.data[i]);
    }

    // C: result += (sum1 - sum2) / 10;  (integer division, truncating toward zero)
    let diff = sum1.wrapping_sub(sum2);
    result = result.wrapping_add(diff / 10);

    // Mirror the local `special` block plus the strcpy that overwrites its
    // name. Neither field-overwrite affects observable output; we keep the
    // structure for fidelity.
    let mut special = DataBlock {
        id: 99,
        name: make_name("Special"),
        flags: 0b1111_1111,
    };
    special.name = make_name("Modified");

    // mem1->data and mem2->data are distinct allocations, so this is true.
    if mem1.data_addr != mem2.data_addr {
        result = result.wrapping_add(special.id);
    }

    // Both mem1->data and mem2->data are non-null pointers (the allocations
    // succeeded), so `> NULL` is true for both, contributing special.flags.
    // special.flags is a u8 (0xFF == 255); promoted to int it adds 255.
    if mem1.data_addr > 0 && mem2.data_addr > 0 {
        result = result.wrapping_add(special.flags as CInt);
    }

    // mem1 and mem2 dropped here (free_block equivalent).
    drop(mem1);
    drop(mem2);

    result
}

fn read_all_stdin() -> String {
    let mut s = String::new();
    let _ = io::stdin().read_to_string(&mut s);
    s
}

/// Parse exactly one integer the way `scanf("%d", ...)` does:
/// skip leading whitespace, then read an optional sign and digits.
/// Returns the integer plus the index just past it. Returns None if no
/// integer was successfully parsed (matches scanf returning < 1).
fn scanf_int(input: &[u8], start: usize) -> Option<(CInt, usize)> {
    let mut i = start;
    while i < input.len() && (input[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    if i >= input.len() {
        return None;
    }
    let mut sign: i64 = 1;
    if input[i] == b'+' {
        i += 1;
    } else if input[i] == b'-' {
        sign = -1;
        i += 1;
    }
    let digits_start = i;
    while i < input.len() && (input[i] as char).is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        return None;
    }
    let mut acc: i64 = 0;
    for &b in &input[digits_start..i] {
        acc = acc.wrapping_mul(10).wrapping_add((b - b'0') as i64);
    }
    Some((acc.wrapping_mul(sign) as CInt, i))
}

fn main() {
    let input = read_all_stdin();
    let bytes = input.as_bytes();

    let mut idx = 0usize;
    let mut vals: [CInt; 4] = [0; 4];
    let mut read_count = 0;
    for slot in vals.iter_mut() {
        match scanf_int(bytes, idx) {
            Some((v, ni)) => {
                *slot = v;
                idx = ni;
                read_count += 1;
            }
            None => break,
        }
    }

    // The original C library has no main; we mirror typical CLI behavior:
    // if four integers were supplied, call betagamma and print the result.
    // Otherwise print nothing (matching a trivially failed scanf), so output
    // remains byte-identical for the well-formed input case used in tests.
    if read_count == 4 {
        let r = betagamma(vals[0], vals[1], vals[2], vals[3]);
        let stdout = io::stdout();
        let mut out = stdout.lock();
        // Match printf("%d\n", r) exactly.
        let _ = writeln!(out, "{}", r);
        let _ = out.flush();
    }
}
