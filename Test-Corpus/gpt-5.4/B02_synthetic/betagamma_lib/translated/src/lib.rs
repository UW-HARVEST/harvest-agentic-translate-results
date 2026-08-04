use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy)]
struct DataBlock {
    id: c_int,
    name: [u8; 32],
    flags: u8,
}

struct MemoryBlock {
    data: Vec<c_int>,
}

fn make_name_bytes(s: &str) -> [u8; 32] {
    let mut name = [0u8; 32];
    let bytes = s.as_bytes();
    let len = bytes.len().min(31);
    name[..len].copy_from_slice(&bytes[..len]);
    name
}

fn create_block(id: c_int, name: &str, flags: u8) -> DataBlock {
    DataBlock {
        id,
        name: make_name_bytes(name),
        flags,
    }
}

fn allocate_block(count: usize, init_value: c_int) -> Option<Box<MemoryBlock>> {
    let mut data = Vec::with_capacity(count);
    for i in 0..count {
        data.push(init_value.wrapping_add(i as c_int));
    }
    Some(Box::new(MemoryBlock { data }))
}

fn compute_hash(mb1: &MemoryBlock, mb2: &MemoryBlock) -> c_int {
    let mut hash = 0;

    let p1 = mb1.data.as_ptr() as usize;
    let p2 = mb2.data.as_ptr() as usize;
    if p1 < p2 {
        hash += 100;
    } else if p1 > p2 {
        hash += 200;
    }

    let m1 = mb1 as *const MemoryBlock as usize;
    let m2 = mb2 as *const MemoryBlock as usize;
    if m1 < m2 {
        hash += 10;
    } else if m1 > m2 {
        hash += 20;
    }

    hash
}

#[unsafe(no_mangle)]
pub extern "C" fn betagamma(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result = 0;

    let blocks = [
        create_block(1, "Block_Alpha", 0b10101010),
        create_block(2, "Block_Beta", 0b11001100),
        create_block(3, "Block_Gamma", 0b11110000),
    ];

    for current in &blocks {
        let mut temp_name = [0u8; 32];
        temp_name.copy_from_slice(&current.name);

        let mut flag_contribution = 0;
        if current.flags & 0b00001111 != 0 {
            flag_contribution += param1;
        }
        if current.flags & 0b11110000 != 0 {
            flag_contribution += param2;
        }
        if current.flags & 0b10101010 != 0 {
            flag_contribution += param3;
        }
        if current.flags & 0b01010101 != 0 {
            flag_contribution += param4;
        }

        result += flag_contribution * current.id;
    }

    let block_size = (param1.rem_euclid(10) as usize) + 5;
    let mem1 = allocate_block(block_size, param1);
    let mem2 = allocate_block(block_size, param2);

    let (mem1, mem2) = match (mem1, mem2) {
        (Some(m1), Some(m2)) => (m1, m2),
        _ => return -1,
    };

    let hash = compute_hash(&mem1, &mem2);
    result += hash;

    let sum1: c_int = mem1.data.iter().copied().sum();
    let sum2: c_int = mem2.data.iter().copied().sum();

    result += (sum1 - sum2) / 10;

    let mut special = create_block(99, "Special", 0b11111111);
    special.name = make_name_bytes("Modified");

    if mem1.data.as_ptr() != mem2.data.as_ptr() {
        result += special.id;
    }

    if !mem1.data.is_empty() || !mem2.data.is_empty() {
        result += special.flags as c_int;
    }

    result
}
