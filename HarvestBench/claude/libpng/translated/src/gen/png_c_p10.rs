/* png.c lines 3769..4044 */

/* HARDWARE OR SOFTWARE OPTION SUPPORT */
/* png_set_option */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_option(
    png_ptr: png_structrp,
    option: c_int,
    onoff: c_int,
) -> c_int {
    if png_ptr != core::ptr::null_mut()
        && option >= 0
        && option < PNG_OPTION_NEXT
        && (option & 1) == 0
    {
        let mask: png_uint_32 = 3u32 << option;
        let setting: png_uint_32 =
            (2u32 + (if onoff != 0 { 1u32 } else { 0u32 })) << option;
        let current: png_uint_32 = (*png_ptr).options;

        (*png_ptr).options = (current & !mask) | setting;

        return ((current & mask) as c_int) >> option;
    }

    PNG_OPTION_INVALID
}

/* SIMPLIFIED READ/WRITE SUPPORT */
/* png_image_free_function */
unsafe extern "C" fn png_image_free_function(argument: png_voidp) -> c_int {
    let image: png_imagep = argument as png_imagep;
    let cp: png_controlp = (*image).opaque;
    let mut c: png_control;

    /* Double check that we have a png_ptr - it should be impossible to get here
     * without one.
     */
    if (*cp).png_ptr == core::ptr::null_mut() {
        return 0;
    }

    /* First free any data held in the control structure. */
    if (*cp).owned_file() != 0 {
        let fp: *mut FILE = (*(*cp).png_ptr).io_ptr as *mut FILE;
        (*cp).set_owned_file(0);

        /* Ignore errors here. */
        if fp != core::ptr::null_mut() {
            (*(*cp).png_ptr).io_ptr = core::ptr::null_mut();
            fclose(fp);
        }
    }

    /* Copy the control structure so that the original, allocated, version can be
     * safely freed.  Notice that a png_error here stops the remainder of the
     * cleanup, but this is probably fine because that would indicate bad memory
     * problems anyway.
     */
    c = core::ptr::read(cp);
    (*image).opaque = &mut c as png_controlp;
    png_free(c.png_ptr, cp as png_voidp);

    /* Then the structures, calling the correct API. */
    if c.for_write() != 0 {
        png_destroy_write_struct(
            core::ptr::addr_of_mut!(c.png_ptr),
            core::ptr::addr_of_mut!(c.info_ptr),
        );
    } else {
        png_destroy_read_struct(
            core::ptr::addr_of_mut!(c.png_ptr),
            core::ptr::addr_of_mut!(c.info_ptr),
            core::ptr::null_mut(),
        );
    }

    /* Success. */
    1
}

/* png_image_free */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_free(image: png_imagep) {
    /* Safely call the real function, but only if doing so is safe at this point
     * (if not inside an error handling context).  Otherwise assume
     * png_safe_execute will call this API after the return.
     */
    if image != core::ptr::null_mut()
        && (*image).opaque != core::ptr::null_mut()
        && (*(*image).opaque).error_buf == core::ptr::null_mut()
    {
        png_image_free_function(image as png_voidp);
        (*image).opaque = core::ptr::null_mut();
    }
}

/* png_image_error */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_error(
    image: png_imagep,
    error_message: png_const_charp,
) -> c_int {
    /* Utility to log an error. */
    png_safecat(
        (*image).message.as_mut_ptr(),
        core::mem::size_of::<[c_char; 64]>(),
        0,
        error_message,
    );
    (*image).warning_or_error |= PNG_IMAGE_ERROR;
    png_image_free(image);
    0
}
