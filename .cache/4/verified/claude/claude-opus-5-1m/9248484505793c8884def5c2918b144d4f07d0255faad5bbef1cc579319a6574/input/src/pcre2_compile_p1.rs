/* Translated from c_src/src/pcre2_compile.c lines 1131-1258 */

/*************************************************
*               Copy compiled code               *
*************************************************/

/* Compiled JIT code cannot be copied, so the new compiled block has no
associated JIT data. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_copy_8(code: *const pcre2_real_code) -> *mut pcre2_real_code {
    let ref_count: *mut PCRE2_SIZE;
    let newcode: *mut pcre2_real_code;

    if code.is_null() {
        return std::ptr::null_mut();
    }
    newcode = ((*code).memctl.malloc.unwrap())((*code).blocksize, (*code).memctl.memory_data)
        as *mut pcre2_real_code;
    if newcode.is_null() {
        return std::ptr::null_mut();
    }
    memcpy(
        newcode as *mut c_void,
        code as *const c_void,
        (*code).blocksize,
    );
    (*newcode).executable_jit = std::ptr::null_mut();

    /* If the code is one that has been deserialized, increment the reference count
    in the decoded tables. */

    if ((*code).flags & PCRE2_DEREF_TABLES) != 0 {
        ref_count = (*code).tables.add(TABLES_LENGTH) as *mut PCRE2_SIZE;
        *ref_count += 1;
    }

    return newcode;
}

/*************************************************
*     Copy compiled code and character tables    *
*************************************************/

/* Compiled JIT code cannot be copied, so the new compiled block has no
associated JIT data. This version of code_copy also makes a separate copy of
the character tables. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_copy_with_tables_8(
    code: *const pcre2_real_code,
) -> *mut pcre2_real_code {
    let ref_count: *mut PCRE2_SIZE;
    let newcode: *mut pcre2_real_code;
    let newtables: *mut u8;

    if code.is_null() {
        return std::ptr::null_mut();
    }
    newcode = ((*code).memctl.malloc.unwrap())((*code).blocksize, (*code).memctl.memory_data)
        as *mut pcre2_real_code;
    if newcode.is_null() {
        return std::ptr::null_mut();
    }
    memcpy(
        newcode as *mut c_void,
        code as *const c_void,
        (*code).blocksize,
    );
    (*newcode).executable_jit = std::ptr::null_mut();

    newtables = ((*code).memctl.malloc.unwrap())(
        TABLES_LENGTH + size_of::<PCRE2_SIZE>(),
        (*code).memctl.memory_data,
    ) as *mut u8;
    if newtables.is_null() {
        ((*code).memctl.free.unwrap())(newcode as *mut c_void, (*code).memctl.memory_data);
        return std::ptr::null_mut();
    }
    memcpy(
        newtables as *mut c_void,
        (*code).tables as *const c_void,
        TABLES_LENGTH,
    );
    ref_count = newtables.add(TABLES_LENGTH) as *mut PCRE2_SIZE;
    *ref_count = 1;

    (*newcode).tables = newtables;
    (*newcode).flags |= PCRE2_DEREF_TABLES;
    return newcode;
}

/*************************************************
*               Free compiled code               *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_free_8(code: *mut pcre2_real_code) {
    let ref_count: *mut PCRE2_SIZE;

    if !code.is_null() {
        /* SUPPORT_JIT is not defined, so there is no executable_jit to free */

        if ((*code).flags & PCRE2_DEREF_TABLES) != 0 {
            /* Decoded tables belong to the codes after deserialization, and they must
            be freed when there are no more references to them. The *ref_count should
            always be > 0. */

            ref_count = (*code).tables.add(TABLES_LENGTH) as *mut PCRE2_SIZE;
            if *ref_count > 0 {
                *ref_count -= 1;
                if *ref_count == 0 {
                    ((*code).memctl.free.unwrap())(
                        (*code).tables as *mut c_void,
                        (*code).memctl.memory_data,
                    );
                }
            }
        }

        ((*code).memctl.free.unwrap())(code as *mut c_void, (*code).memctl.memory_data);
    }
}
