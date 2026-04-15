use std::os::raw::c_int;

struct DataBlock {
    id: c_int,
    name: String,
    flags: u8,
}

struct MemoryBlock {
    data: Vec<c_int>,
}

#[allow(dead_code)]
fn create_block(id: c_int, name: &str, flags: u8) -> DataBlock {
    DataBlock {
        id,
        name: name.to_string(),
        flags,
    }
}

fn allocate_block(count: usize, init_value: c_int) -> Option<Box<MemoryBlock>> {
    let mut data = Vec::new();
    if data.try_reserve(count).is_err() {
        return None;
    }
    data.resize(count, 0);
    for i in 0..count {
        data[i] = init_value.wrapping_add(i as c_int);
    }
    Some(Box::new(MemoryBlock { data }))
}

#[allow(dead_code)]
fn free_block(_mb: Option<Box<MemoryBlock>>) {
    // Memory is automatically freed in Rust when dropped
}

fn compute_hash(mb1: &MemoryBlock, mb2: &MemoryBlock) -> c_int {
    let mut hash = 0;

    let ptr1 = mb1.data.as_ptr() as usize;
    let ptr2 = mb2.data.as_ptr() as usize;

    if ptr1 < ptr2 {
        hash += 100;
    } else if ptr1 > ptr2 {
        hash += 200;
    }

    let mb1_ptr = mb1 as *const MemoryBlock as usize;
    let mb2_ptr = mb2 as *const MemoryBlock as usize;

    if mb1_ptr < mb2_ptr {
        hash += 10;
    } else if mb1_ptr > mb2_ptr {
        hash += 20;
    }

    hash
}

#[unsafe(no_mangle)]
pub extern "C" fn betagamma(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let blocks = [
        DataBlock { id: 1, name: "Block_Alpha".to_string(), flags: 0b10101010 },
        DataBlock { id: 2, name: "Block_Beta".to_string(), flags: 0b11001100 },
        DataBlock { id: 3, name: "Block_Gamma".to_string(), flags: 0b11110000 },
    ];

    for current in &blocks {
        let _temp_name = current.name.clone();

        let mut flag_contribution: c_int = 0;
        if (current.flags & 0b00001111) != 0 {
            flag_contribution = flag_contribution.wrapping_add(param1);
        }
        if (current.flags & 0b11110000) != 0 {
            flag_contribution = flag_contribution.wrapping_add(param2);
        }
        if (current.flags & 0b10101010) != 0 {
            flag_contribution = flag_contribution.wrapping_add(param3);
        }
        if (current.flags & 0b01010101) != 0 {
            flag_contribution = flag_contribution.wrapping_add(param4);
        }

        result = result.wrapping_add(flag_contribution.wrapping_mul(current.id));
    }

    let block_size = ((param1 % 10) + 5) as usize;
    let mem1 = allocate_block(block_size, param1);
    let mem2 = allocate_block(block_size, param2);

    let (mem1, mem2) = match (mem1, mem2) {
        (Some(m1), Some(m2)) => (m1, m2),
        _ => return -1,
    };

    let hash = compute_hash(&mem1, &mem2);
    result = result.wrapping_add(hash);

    let mut sum1: c_int = 0;
    let mut sum2: c_int = 0;
    for &val in &mem1.data {
        sum1 = sum1.wrapping_add(val);
    }
    for &val in &mem2.data {
        sum2 = sum2.wrapping_add(val);
    }

    result = result.wrapping_add(sum1.wrapping_sub(sum2) / 10);

    let mut special = DataBlock {
        id: 99,
        name: "Special".to_string(),
        flags: 0b11111111,
    };
    special.name = "Modified".to_string();

    if mem1.data.as_ptr() != mem2.data.as_ptr() {
        result = result.wrapping_add(special.id);
    }

    if (mem1.data.as_ptr() as usize) > 0 && (mem2.data.as_ptr() as usize) > 0 {
        result = result.wrapping_add(special.flags as c_int);
    }

    result
}
