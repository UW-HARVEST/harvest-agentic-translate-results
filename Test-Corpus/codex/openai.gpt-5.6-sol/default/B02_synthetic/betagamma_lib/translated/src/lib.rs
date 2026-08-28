use std::ffi::{c_char, c_int, c_void};
use std::mem::{MaybeUninit, size_of};
use std::ptr::{addr_of_mut, null_mut};

#[repr(C)]
pub struct DataBlock {
    pub id: c_int,
    pub name: [c_char; 32],
    pub flags: u8,
}

#[repr(C)]
pub struct MemoryBlock {
    pub data: *mut c_int,
    pub size: usize,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_block(id: c_int, name: *const c_char, flags: u8) -> DataBlock {
    let mut block = MaybeUninit::<DataBlock>::uninit();
    let block_ptr = block.as_mut_ptr();

    // SAFETY: This intentionally has the same caller requirements as the C function.
    unsafe {
        addr_of_mut!((*block_ptr).id).write(id);
        strcpy(addr_of_mut!((*block_ptr).name).cast::<c_char>(), name);
        addr_of_mut!((*block_ptr).flags).write(flags);
        block.assume_init()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_block(count: usize, init_value: c_int) -> *mut MemoryBlock {
    // SAFETY: The returned allocations are checked before they are dereferenced.
    let block = unsafe { malloc(size_of::<MemoryBlock>()) }.cast::<MemoryBlock>();
    if block.is_null() {
        return null_mut();
    }

    // SAFETY: block points to enough storage for MemoryBlock.
    unsafe {
        addr_of_mut!((*block).data).write(calloc(count, size_of::<c_int>()).cast::<c_int>());
        if (*block).data.is_null() {
            free(block.cast::<c_void>());
            return null_mut();
        }

        addr_of_mut!((*block).size).write(count);
        for i in 0..count {
            (*block)
                .data
                .add(i)
                .write((init_value as usize).wrapping_add(i) as c_int);
        }
    }

    block
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_block(block: *mut MemoryBlock) {
    if !block.is_null() {
        // SAFETY: This intentionally accepts allocations under the same contract as the C API.
        unsafe {
            if !(*block).data.is_null() {
                free((*block).data.cast::<c_void>());
            }
            free(block.cast::<c_void>());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_hash(block1: *mut MemoryBlock, block2: *mut MemoryBlock) -> c_int {
    // SAFETY: The C API requires two valid MemoryBlock pointers.
    let (data1, data2) = unsafe { ((*block1).data.addr(), (*block2).data.addr()) };
    let mut hash: c_int = 0;

    if data1 < data2 {
        hash += 100;
    } else if data1 > data2 {
        hash += 200;
    }

    if block1.addr() < block2.addr() {
        hash += 10;
    } else if block1.addr() > block2.addr() {
        hash += 20;
    }

    hash
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn betagamma(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let blocks = [
        DataBlock {
            id: 1,
            name: c_name(b"Block_Alpha\0"),
            flags: 0b10101010,
        },
        DataBlock {
            id: 2,
            name: c_name(b"Block_Beta\0"),
            flags: 0b11001100,
        },
        DataBlock {
            id: 3,
            name: c_name(b"Block_Gamma\0"),
            flags: 0b11110000,
        },
    ];

    let mut result: c_int = 0;
    for current in &blocks {
        let mut temp_name = [0 as c_char; 32];
        // SAFETY: Both arrays are NUL-terminated and have the same capacity.
        unsafe {
            strcpy(temp_name.as_mut_ptr(), current.name.as_ptr());
        }

        let mut flag_contribution: c_int = 0;
        if current.flags & 0b00001111 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param1);
        }
        if current.flags & 0b11110000 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param2);
        }
        if current.flags & 0b10101010 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param3);
        }
        if current.flags & 0b01010101 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param4);
        }

        result = result.wrapping_add(flag_contribution.wrapping_mul(current.id));
    }

    let block_size = (param1 % 10).wrapping_add(5) as usize;
    // SAFETY: allocate_block has no additional caller requirements.
    let mem1 = unsafe { allocate_block(block_size, param1) };
    let mem2 = unsafe { allocate_block(block_size, param2) };

    if mem1.is_null() || mem2.is_null() {
        // SAFETY: free_block accepts null.
        unsafe {
            free_block(mem1);
            free_block(mem2);
        }
        return -1;
    }

    // SAFETY: Both allocations were checked above.
    let hash = unsafe { compute_hash(mem1, mem2) };
    result = result.wrapping_add(hash);

    let mut sum1: c_int = 0;
    let mut sum2: c_int = 0;
    // SAFETY: allocate_block initialized each data array to its recorded size.
    unsafe {
        for i in 0..(*mem1).size {
            sum1 = sum1.wrapping_add(*(*mem1).data.add(i));
        }
        for i in 0..(*mem2).size {
            sum2 = sum2.wrapping_add(*(*mem2).data.add(i));
        }
    }
    result = result.wrapping_add(sum1.wrapping_sub(sum2) / 10);

    let mut special = DataBlock {
        id: 99,
        name: c_name(b"Special\0"),
        flags: 0b11111111,
    };
    // SAFETY: "Modified" and its terminator fit in special.name.
    unsafe {
        strcpy(special.name.as_mut_ptr(), c"Modified".as_ptr());
    }

    // SAFETY: Both MemoryBlock pointers are valid until the final frees.
    unsafe {
        if (*mem1).data != (*mem2).data {
            result = result.wrapping_add(special.id);
        }

        if (*mem1).data.addr() > 0 && (*mem2).data.addr() > 0 {
            result = result.wrapping_add(c_int::from(special.flags));
        }

        free_block(mem1);
        free_block(mem2);
    }

    result
}

const fn c_name<const N: usize>(value: &[u8; N]) -> [c_char; 32] {
    let mut result = [0 as c_char; 32];
    let mut i = 0;
    while i < N {
        result[i] = value[i] as c_char;
        i += 1;
    }
    result
}
