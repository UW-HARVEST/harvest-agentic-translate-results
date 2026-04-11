extern "C" {
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn realloc(__ptr: *mut libc::c_void, __size: size_t) -> *mut libc::c_void;
    fn strncpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
        __n: size_t,
    ) -> *mut libc::c_char;
    fn strdup(__s: *const libc::c_char) -> *mut libc::c_char;
    fn strstr(
        __haystack: *const libc::c_char,
        __needle: *const libc::c_char,
    ) -> *mut libc::c_char;
    fn strlen(__s: *const libc::c_char) -> size_t;
}
pub type size_t = usize;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn searchAndReplace(
    mut orig: *const libc::c_char,
    mut search: *const libc::c_char,
    mut value: *const libc::c_char,
) -> *mut libc::c_char {
    let mut p: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let orig_len: size_t = strlen(orig) as size_t;
    let search_len: size_t = strlen(search) as size_t;
    let value_len: size_t = strlen(value) as size_t;
    let mut inx_start: size_t = 0;
    let mut tmp: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut tmp_offset: size_t = 0 as size_t;
    let mut total_bytes_allocated: size_t = 1 as size_t;
    let mut from: size_t = 0;
    p = strstr(orig, search);
    if p.is_null() {
        tmp = strdup(orig);
        return tmp;
    }
    inx_start = p.offset_from(orig) as libc::c_long as size_t;
    from = inx_start.wrapping_add(search_len);
    if inx_start > 0 as size_t {
        total_bytes_allocated = inx_start.wrapping_add(1 as size_t);
        tmp = malloc(
            (std::mem::size_of::<libc::c_char>() as size_t)
                .wrapping_mul(total_bytes_allocated),
        ) as *mut libc::c_char;
        if tmp.is_null() {
            return std::ptr::null_mut::<libc::c_char>();
        }
        strncpy(tmp, orig, inx_start);
        tmp_offset = inx_start;
    }
    while !p.is_null() {
        total_bytes_allocated = (total_bytes_allocated as libc::c_ulong)
            .wrapping_add(value_len as libc::c_ulong)
            as size_t as size_t;
        tmp = realloc(tmp as *mut libc::c_void, total_bytes_allocated)
            as *mut libc::c_char;
        if tmp.is_null() {
            return std::ptr::null_mut::<libc::c_char>();
        }
        strncpy(
            tmp.offset(tmp_offset as isize),
            value,
            total_bytes_allocated.wrapping_sub(tmp_offset),
        );
        tmp_offset = (tmp_offset as libc::c_ulong)
            .wrapping_add(value_len as libc::c_ulong) as size_t
            as size_t;
        p = strstr(
            orig.offset(inx_start as isize).offset(search_len as isize),
            search,
        );
        if !p.is_null() {
            let mut inx_start2: size_t = p.offset_from(orig) as libc::c_long as size_t;
            if inx_start2 > from {
                let mut gap: size_t = inx_start2.wrapping_sub(from);
                total_bytes_allocated = (total_bytes_allocated as libc::c_ulong)
                    .wrapping_add(gap as libc::c_ulong)
                    as size_t as size_t;
                tmp = realloc(tmp as *mut libc::c_void, total_bytes_allocated)
                    as *mut libc::c_char;
                if tmp.is_null() {
                    return std::ptr::null_mut::<libc::c_char>();
                }
                strncpy(
                    tmp.offset(tmp_offset as isize),
                    orig.offset(from as isize),
                    gap,
                );
                tmp_offset = (tmp_offset as libc::c_ulong)
                    .wrapping_add(gap as libc::c_ulong)
                    as size_t as size_t;
            }
            inx_start = inx_start2;
        }
        from = inx_start.wrapping_add(search_len);
    }
    if from < orig_len && from > 0 as size_t {
        total_bytes_allocated = (total_bytes_allocated as libc::c_ulong)
            .wrapping_add(orig_len.wrapping_sub(from) as libc::c_ulong)
            as size_t as size_t;
        tmp = realloc(tmp as *mut libc::c_void, total_bytes_allocated)
            as *mut libc::c_char;
        if tmp.is_null() {
            return std::ptr::null_mut::<libc::c_char>();
        }
        strncpy(
            tmp.offset(tmp_offset as isize),
            orig.offset(from as isize),
            orig_len.wrapping_sub(from),
        );
    }
    *tmp.offset(total_bytes_allocated.wrapping_sub(1 as size_t) as isize) =
        '\0' as i32 as libc::c_char;
    return tmp;
}
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}

