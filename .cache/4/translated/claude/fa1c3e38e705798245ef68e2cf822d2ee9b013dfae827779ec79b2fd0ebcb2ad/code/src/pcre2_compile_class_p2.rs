/* Translated from c_src/src/pcre2_compile_class.c lines 751-1071 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_update_classbits_8(
    ptype: u32,
    pdata: u32,
    negated: BOOL,
    classbits: *mut u8,
) {
    /* Update PRIV(xclass) when this function is changed. */
    let mut classbits: *mut u8 = classbits;
    let mut c: c_int;
    let mut chartype: c_int;
    let mut prop: *const ucd_record;
    let mut gentype: u32;
    let mut set_bit: BOOL;

    if ptype == PT_ANY {
        if negated == 0 {
            memset(classbits as *mut c_void, 0xff, 32);
        }
        return;
    }

    c = 0;
    while c < 256 {
        prop = GET_UCD(c as u32);
        set_bit = FALSE;

        match ptype {
            PT_LAMP => {
                chartype = (*prop).chartype as c_int;
                set_bit = (chartype == ucp_Lu as c_int
                    || chartype == ucp_Ll as c_int
                    || chartype == ucp_Lt as c_int) as BOOL;
            }

            PT_GC => {
                set_bit = (*_pcre2_ucp_gentype_8
                    .as_ptr()
                    .add((*prop).chartype as usize)
                    == pdata) as BOOL;
            }

            PT_PC => {
                set_bit = ((*prop).chartype as u32 == pdata) as BOOL;
            }

            PT_SC => {
                set_bit = ((*prop).script as u32 == pdata) as BOOL;
            }

            PT_SCX => {
                set_bit = ((*prop).script as u32 == pdata
                    || MAPBIT!(
                        _pcre2_ucd_script_sets_8
                            .as_ptr()
                            .add(UCD_SCRIPTX_PROP(prop) as usize),
                        pdata
                    ) != 0) as BOOL;
            }

            PT_ALNUM => {
                gentype = *_pcre2_ucp_gentype_8
                    .as_ptr()
                    .add((*prop).chartype as usize);
                set_bit = (gentype == ucp_L || gentype == ucp_N) as BOOL;
            }

            PT_SPACE /* Perl space */ | PT_PXSPACE /* POSIX space */ => {
                match c as u32 {
                    /* HSPACE_BYTE_CASES */
                    CHAR_HT
                    | CHAR_SPACE
                    | CHAR_NBSP
                    /* VSPACE_BYTE_CASES */
                    | CHAR_LF
                    | CHAR_VT
                    | CHAR_FF
                    | CHAR_CR
                    | CHAR_NEL => {
                        set_bit = TRUE;
                    }

                    _ => {
                        set_bit = (*_pcre2_ucp_gentype_8
                            .as_ptr()
                            .add((*prop).chartype as usize)
                            == ucp_Z) as BOOL;
                    }
                }
            }

            PT_WORD => {
                chartype = (*prop).chartype as c_int;
                gentype = *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize);
                set_bit = (gentype == ucp_L
                    || gentype == ucp_N
                    || chartype == ucp_Mn as c_int
                    || chartype == ucp_Pc as c_int) as BOOL;
            }

            PT_UCNC => {
                set_bit = (c as u32 == CHAR_DOLLAR_SIGN
                    || c as u32 == CHAR_COMMERCIAL_AT
                    || c as u32 == CHAR_GRAVE_ACCENT
                    || c >= 0xa0) as BOOL;
            }

            PT_BIDICL => {
                set_bit = (UCD_BIDICLASS_PROP(prop) == pdata) as BOOL;
            }

            PT_BOOL => {
                set_bit = (MAPBIT!(
                    _pcre2_ucd_boolprop_sets_8
                        .as_ptr()
                        .add(UCD_BPROPS_PROP(prop) as usize),
                    pdata
                ) != 0) as BOOL;
            }

            PT_PXGRAPH => {
                chartype = (*prop).chartype as c_int;
                gentype = *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize);
                set_bit = (gentype != ucp_Z
                    && (gentype != ucp_C || chartype == ucp_Cf as c_int)) as BOOL;
            }

            PT_PXPRINT => {
                chartype = (*prop).chartype as c_int;
                set_bit = (chartype != ucp_Zl as c_int
                    && chartype != ucp_Zp as c_int
                    && (*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) != ucp_C
                        || chartype == ucp_Cf as c_int)) as BOOL;
            }

            PT_PXPUNCT => {
                gentype = *_pcre2_ucp_gentype_8
                    .as_ptr()
                    .add((*prop).chartype as usize);
                set_bit = (gentype == ucp_P || (c < 128 && gentype == ucp_S)) as BOOL;
            }

            _ => {
                /* PCRE2_ASSERT(ptype == PT_PXXDIGIT); */
                set_bit = ((c as u32 >= CHAR_0 && c as u32 <= CHAR_9)
                    || (c as u32 >= CHAR_A && c as u32 <= CHAR_F)
                    || (c as u32 >= CHAR_a && c as u32 <= CHAR_f)) as BOOL;
            }
        }

        if negated != 0 {
            set_bit = (set_bit == 0) as BOOL;
        }
        if set_bit != 0 {
            *classbits |= (1u32 << (c & 0x7)) as u8;
        }
        if (c & 0x7) == 0x7 {
            classbits = classbits.add(1);
        }

        c += 1;
    }
}

/*************************************************
*   Internal entry point for add range to class  *
*************************************************/

/* This function sets the overall range for characters < 256.
It also handles non-utf case folding.

Arguments:
  options       the options bits
  xoptions      the extra options bits
  cb            compile data
  start         start of range character
  end           end of range character

Returns:        cb->classbits is updated
*/

unsafe fn add_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    start: u32,
    end: u32,
) {
    let classbits: *mut u8 = (*cb).classbits.classbits.as_mut_ptr();
    let mut c: u32;
    let mut byte_start: u32;
    let mut byte_end: u32;
    let classbits_end: u32 = if end <= 0xff { end } else { 0xff };

    /* If caseless matching is required, scan the range and process alternate
    cases. In Unicode, there are 8-bit characters that have alternate cases that
    are greater than 255 and vice-versa (though these may be ignored if caseless
    restriction is in force). Sometimes we can just extend the original range. */

    if (options & PCRE2_CASELESS) != 0 {
        /* UTF mode. This branch is taken if we don't support wide characters (e.g.
        8-bit library, without UTF), but we do treat those characters as Unicode
        (if UCP flag is set). In this case, we only need to expand the character class
        set to include the case pairs which are in the 0-255 codepoint range. */
        if (options & (PCRE2_UTF | PCRE2_UCP)) != 0 {
            let turkish_i: BOOL = ((xoptions
                & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
                == PCRE2_EXTRA_TURKISH_CASING) as BOOL;
            if start < 128 {
                let lo_end: u32 = if classbits_end < 127 { classbits_end } else { 127 };
                c = start;
                while c <= lo_end {
                    if turkish_i != 0 && UCD_ANY_I(c) {
                        c += 1;
                        continue;
                    }
                    SETBIT!(classbits, *(*cb).fcc.add(c as usize) as u32);
                    c += 1;
                }
            }
            if classbits_end >= 128 {
                let hi_start: u32 = if start > 128 { start } else { 128 };
                c = hi_start;
                while c <= classbits_end {
                    let co: u32 = UCD_OTHERCASE(c);
                    if co <= 0xff {
                        SETBIT!(classbits, co);
                    }
                    c += 1;
                }
            }
        }
        /* Not UTF mode */
        else {
            c = start;
            while c <= classbits_end {
                SETBIT!(classbits, *(*cb).fcc.add(c as usize) as u32);
                c += 1;
            }
        }
    }

    /* Use the bitmap for characters < 256. Otherwise use extra data. */

    byte_start = (start + 7) >> 3;
    byte_end = (classbits_end + 1) >> 3;

    if byte_start >= byte_end {
        c = start;
        while c <= classbits_end {
            /* Regardless of start, c will always be <= 255. */
            SETBIT!(classbits, c);
            c += 1;
        }
        return;
    }

    c = byte_start;
    while c < byte_end {
        *classbits.add(c as usize) = 0xff;
        c += 1;
    }

    byte_start <<= 3;
    byte_end <<= 3;

    c = start;
    while c < byte_start {
        SETBIT!(classbits, c);
        c += 1;
    }

    c = byte_end;
    while c <= classbits_end {
        SETBIT!(classbits, c);
        c += 1;
    }
}

/*************************************************
*   Internal entry point for add list to class   *
*************************************************/

/* This function is used for adding a list of horizontal or vertical whitespace
characters to a class. The list must be in order so that ranges of characters
can be detected and handled appropriately. This function sets the overall range
so that the internal functions can try to avoid duplication when handling
case-independence.

Arguments:
  options       the options bits
  xoptions      the extra options bits
  cb            contains pointers to tables etc.
  p             points to row of 32-bit values, terminated by NOTACHAR

Returns:        cb->classbits is updated
*/

unsafe fn add_list_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    p: *const u32,
) {
    let mut p: *const u32 = p;

    while *p < 256 {
        let mut n: c_uint = 0;

        while *p.add((n + 1) as usize) == *p + n + 1 {
            n += 1;
        }
        add_to_class(options, xoptions, cb, *p, *p.add(n as usize));

        p = p.add((n + 1) as usize);
    }
}

/*************************************************
*    Add characters not in a list to a class     *
*************************************************/

/* This function is used for adding the complement of a list of horizontal or
vertical whitespace to a class. The list must be in order.

Arguments:
  options       the options bits
  xoptions      the extra options bits
  cb            contains pointers to tables etc.
  p             points to row of 32-bit values, terminated by NOTACHAR

Returns:        cb->classbits is updated
*/

unsafe fn add_not_list_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    p: *const u32,
) {
    let mut p: *const u32 = p;

    if *p > 0 {
        add_to_class(options, xoptions, cb, 0, *p - 1);
    }
    while *p < 256 {
        while *p.add(1) == *p + 1 {
            p = p.add(1);
        }
        add_to_class(
            options,
            xoptions,
            cb,
            *p + 1,
            if *p.add(1) > 255 { 255 } else { *p.add(1) - 1 },
        );
        p = p.add(1);
    }
}
