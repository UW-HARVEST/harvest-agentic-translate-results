extern "C" {
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn strcpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
    ) -> *mut libc::c_char;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type uint8_t = __uint8_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DataBlock {
    pub id: libc::c_int,
    pub name: [libc::c_char; 32],
    pub flags: uint8_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct MemoryBlock {
    pub data: *mut libc::c_int,
    pub size: size_t,
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn create_block(
    mut id: libc::c_int,
    mut name: *const libc::c_char,
    mut flags: uint8_t,
) -> DataBlock {
    let mut block: DataBlock = DataBlock {
        id: 0,
        name: [0; 32],
        flags: 0,
    };
    block.id = id;
    strcpy(&raw mut block.name as *mut libc::c_char, name);
    block.flags = flags;
    return block;
}
#[no_mangle]
pub unsafe extern "C" fn allocate_block(
    mut count: size_t,
    mut init_value: libc::c_int,
) -> *mut MemoryBlock {
    let mut mb: *mut MemoryBlock =
        malloc(std::mem::size_of::<MemoryBlock>() as size_t) as *mut MemoryBlock;
    if mb.is_null() {
        return std::ptr::null_mut::<MemoryBlock>();
    }
    (*mb).data = calloc(
        count,
        std::mem::size_of::<libc::c_int>() as size_t,
    ) as *mut libc::c_int;
    if (*mb).data.is_null() {
        free(mb as *mut libc::c_void);
        return std::ptr::null_mut::<MemoryBlock>();
    }
    (*mb).size = count;
    let mut i: size_t = 0 as size_t;
    while i < count {
        *(*mb).data.offset(i as isize) =
            (init_value as size_t).wrapping_add(i) as libc::c_int;
        i = i.wrapping_add(1);
    }
    return mb;
}
#[no_mangle]
pub unsafe extern "C" fn free_block(mut mb: *mut MemoryBlock) {
    if !mb.is_null() {
        if !(*mb).data.is_null() {
            free((*mb).data as *mut libc::c_void);
        }
        free(mb as *mut libc::c_void);
    }
}
#[no_mangle]
pub unsafe extern "C" fn compute_hash(
    mut mb1: *mut MemoryBlock,
    mut mb2: *mut MemoryBlock,
) -> libc::c_int {
    let mut hash: libc::c_int = 0 as libc::c_int;
    if (*mb1).data < (*mb2).data {
        hash += 100 as libc::c_int;
    } else if (*mb1).data > (*mb2).data {
        hash += 200 as libc::c_int;
    }
    if mb1 < mb2 {
        hash += 10 as libc::c_int;
    } else if mb1 > mb2 {
        hash += 20 as libc::c_int;
    }
    return hash;
}
#[no_mangle]
pub unsafe extern "C" fn betagamma(
    mut param1: libc::c_int,
    mut param2: libc::c_int,
    mut param3: libc::c_int,
    mut param4: libc::c_int,
) -> libc::c_int {
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut blocks: [DataBlock; 3] = [
        DataBlock {
            id: 1 as libc::c_int,
            name: std::mem::transmute::<[u8; 32], [libc::c_char; 32]>(
                *b"Block_Alpha\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            ),
            flags: 0o252 as uint8_t,
        },
        DataBlock {
            id: 2 as libc::c_int,
            name: std::mem::transmute::<[u8; 32], [libc::c_char; 32]>(
                *b"Block_Beta\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            ),
            flags: 0o314 as uint8_t,
        },
        DataBlock {
            id: 3 as libc::c_int,
            name: std::mem::transmute::<[u8; 32], [libc::c_char; 32]>(
                *b"Block_Gamma\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            ),
            flags: 0o360 as uint8_t,
        },
    ];
    let mut num_blocks: libc::c_int = (std::mem::size_of::<[DataBlock; 3]>() as usize)
        .wrapping_div(std::mem::size_of::<DataBlock>() as usize)
        as libc::c_int;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < num_blocks {
        let mut current: *mut DataBlock =
            (&raw mut blocks as *mut DataBlock).offset(i as isize) as *mut DataBlock;
        let mut temp_name: [libc::c_char; 32] = [0; 32];
        strcpy(
            &raw mut temp_name as *mut libc::c_char,
            &raw mut (*current).name as *mut libc::c_char,
        );
        let mut flag_contribution: libc::c_int = 0 as libc::c_int;
        if (*current).flags as libc::c_int & 0o17 as libc::c_int != 0 {
            flag_contribution += param1;
        }
        if (*current).flags as libc::c_int & 0o360 as libc::c_int != 0 {
            flag_contribution += param2;
        }
        if (*current).flags as libc::c_int & 0o252 as libc::c_int != 0 {
            flag_contribution += param3;
        }
        if (*current).flags as libc::c_int & 0o125 as libc::c_int != 0 {
            flag_contribution += param4;
        }
        result += flag_contribution * (*current).id;
        i += 1;
    }
    let mut block_size: size_t =
        (param1 % 10 as libc::c_int + 5 as libc::c_int) as size_t;
    let mut mem1: *mut MemoryBlock = allocate_block(block_size, param1);
    let mut mem2: *mut MemoryBlock = allocate_block(block_size, param2);
    if mem1.is_null() || mem2.is_null() {
        free_block(mem1);
        free_block(mem2);
        return -(1 as libc::c_int);
    }
    let mut hash: libc::c_int = compute_hash(mem1, mem2);
    result += hash;
    let mut sum1: libc::c_int = 0 as libc::c_int;
    let mut sum2: libc::c_int = 0 as libc::c_int;
    let mut i_0: size_t = 0 as size_t;
    while i_0 < (*mem1).size {
        sum1 += *(*mem1).data.offset(i_0 as isize);
        i_0 = i_0.wrapping_add(1);
    }
    let mut i_1: size_t = 0 as size_t;
    while i_1 < (*mem2).size {
        sum2 += *(*mem2).data.offset(i_1 as isize);
        i_1 = i_1.wrapping_add(1);
    }
    result += (sum1 - sum2) / 10 as libc::c_int;
    let mut special: DataBlock = DataBlock {
        id: 99 as libc::c_int,
        name: std::mem::transmute::<[u8; 32], [libc::c_char; 32]>(
            *b"Special\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
        ),
        flags: 0o377 as uint8_t,
    };
    strcpy(
        &raw mut special.name as *mut libc::c_char,
        b"Modified\0" as *const u8 as *const libc::c_char,
    );
    if (*mem1).data != (*mem2).data {
        result += special.id;
    }
    if (*mem1).data > NULL as *mut libc::c_int
        && (*mem2).data > NULL as *mut libc::c_int
    {
        result += special.flags as libc::c_int;
    }
    free_block(mem1);
    free_block(mem2);
    return result;
}
