// Translations of the C macros from pcre2_internal.h / pcre2_intmodedep.h /
// pcre2_compile.h for the 8-bit library with LINK_SIZE == 2 and
// SUPPORT_UNICODE defined.
#![allow(unused_macros)]

/* ---------------- Link and 16-bit value access (8-bit mode, LINK_SIZE 2) ---------- */

/// GET(a,n): read a LINK_SIZE value at a[n]; result is `u32`.
#[macro_export]
macro_rules! GET {
    ($a:expr, $n:expr) => {
        ((*$a.add($n as usize) as u32) << 8) | (*$a.add($n as usize + 1) as u32)
    };
}

/// PUT(a,n,d): store a LINK_SIZE value at a[n].
#[macro_export]
macro_rules! PUT {
    ($a:expr, $n:expr, $d:expr) => {{
        let d_ = $d as u32;
        *$a.add($n as usize) = (d_ >> 8) as u8;
        *$a.add($n as usize + 1) = (d_ & 255) as u8;
    }};
}

/// GET2(a,n): read a 2-byte (IMM2_SIZE) value at a[n]; result is `u32`.
#[macro_export]
macro_rules! GET2 {
    ($a:expr, $n:expr) => {
        ((*$a.add($n as usize) as u32) << 8) | (*$a.add($n as usize + 1) as u32)
    };
}

/// PUT2(a,n,d): store a 2-byte value at a[n].
#[macro_export]
macro_rules! PUT2 {
    ($a:expr, $n:expr, $d:expr) => {{
        let d_ = $d as u32;
        *$a.add($n as usize) = (d_ >> 8) as u8;
        *$a.add($n as usize + 1) = (d_ & 255) as u8;
    }};
}

/// PUTINC(a,n,d): PUT then advance `a` by LINK_SIZE.
#[macro_export]
macro_rules! PUTINC {
    ($a:expr, $n:expr, $d:expr) => {{
        PUT!($a, $n, $d);
        $a = $a.add(crate::consts::LINK_SIZE);
    }};
}

/// PUT2INC(a,n,d): PUT2 then advance `a` by IMM2_SIZE.
#[macro_export]
macro_rules! PUT2INC {
    ($a:expr, $n:expr, $d:expr) => {{
        PUT2!($a, $n, $d);
        $a = $a.add(crate::consts::IMM2_SIZE);
    }};
}

/* ---------------- Table access ---------------- */

/// TABLE_GET(c, table, default) - in 8-bit mode there is no range check.
#[macro_export]
macro_rules! TABLE_GET {
    ($c:expr, $table:expr, $default:expr) => {
        *$table.add($c as usize)
    };
}

/// MAX_255(c) is TRUE in 8-bit mode.
#[macro_export]
macro_rules! MAX_255 {
    ($c:expr) => {
        1
    };
}

/// CHMAX_255(c): c <= 255 (8-bit mode with Unicode support).
#[macro_export]
macro_rules! CHMAX_255 {
    ($c:expr) => {
        (($c as u32) <= 255u32) as crate::types::BOOL
    };
}

/// SETBIT(a,b): set bit b in the byte array a.
#[macro_export]
macro_rules! SETBIT {
    ($a:expr, $b:expr) => {
        *$a.add(($b as usize) >> 3) |= (1u32 << (($b as u32) & 0x7)) as u8
    };
}

/// MAPBIT(map,n) - test a bit in a 32-bit word bitmap; nonzero if set.
#[macro_export]
macro_rules! MAPBIT {
    ($map:expr, $n:expr) => {
        (*$map.add(($n as usize) / 32) & (1u32 << (($n as u32) % 32)))
    };
}

/// MAPSET(map,n) - set a bit in a 32-bit word bitmap.
#[macro_export]
macro_rules! MAPSET {
    ($map:expr, $n:expr) => {
        *$map.add(($n as usize) / 32) |= 1u32 << (($n as u32) % 32)
    };
}

/* ---------------- UTF-8 handling ---------------- */

/// HASUTF8EXTRALEN / HAS_EXTRALEN
#[macro_export]
macro_rules! HAS_EXTRALEN {
    ($c:expr) => {
        ($c as u32) >= 0xc0u32
    };
}

/// GET_EXTRALEN(c)
#[macro_export]
macro_rules! GET_EXTRALEN {
    ($c:expr) => {
        crate::tables::_pcre2_utf8_table4[(($c as u32) & 0x3f) as usize] as u32
    };
}

/// NOT_FIRSTCU(c)
#[macro_export]
macro_rules! NOT_FIRSTCU {
    ($c:expr) => {
        (($c as u32) & 0xc0u32) == 0x80u32
    };
}

/// GETUTF8(c, eptr): complete the character in `c` without advancing `eptr`.
#[macro_export]
macro_rules! GETUTF8 {
    ($c:expr, $eptr:expr) => {{
        if ($c & 0x20u32) == 0 {
            $c = (($c & 0x1fu32) << 6) | (*$eptr.add(1) as u32 & 0x3fu32);
        } else if ($c & 0x10u32) == 0 {
            $c = (($c & 0x0fu32) << 12)
                | ((*$eptr.add(1) as u32 & 0x3fu32) << 6)
                | (*$eptr.add(2) as u32 & 0x3fu32);
        } else if ($c & 0x08u32) == 0 {
            $c = (($c & 0x07u32) << 18)
                | ((*$eptr.add(1) as u32 & 0x3fu32) << 12)
                | ((*$eptr.add(2) as u32 & 0x3fu32) << 6)
                | (*$eptr.add(3) as u32 & 0x3fu32);
        } else if ($c & 0x04u32) == 0 {
            $c = (($c & 0x03u32) << 24)
                | ((*$eptr.add(1) as u32 & 0x3fu32) << 18)
                | ((*$eptr.add(2) as u32 & 0x3fu32) << 12)
                | ((*$eptr.add(3) as u32 & 0x3fu32) << 6)
                | (*$eptr.add(4) as u32 & 0x3fu32);
        } else {
            $c = (($c & 0x01u32) << 30)
                | ((*$eptr.add(1) as u32 & 0x3fu32) << 24)
                | ((*$eptr.add(2) as u32 & 0x3fu32) << 18)
                | ((*$eptr.add(3) as u32 & 0x3fu32) << 12)
                | ((*$eptr.add(4) as u32 & 0x3fu32) << 6)
                | (*$eptr.add(5) as u32 & 0x3fu32);
        }
    }};
}

/// GETUTF8INC(c, eptr): complete the character in `c`, advancing `eptr`.
#[macro_export]
macro_rules! GETUTF8INC {
    ($c:expr, $eptr:expr) => {{
        if ($c & 0x20u32) == 0 {
            let t_ = *$eptr as u32;
            $eptr = $eptr.add(1);
            $c = (($c & 0x1fu32) << 6) | (t_ & 0x3fu32);
        } else if ($c & 0x10u32) == 0 {
            $c = (($c & 0x0fu32) << 12)
                | ((*$eptr as u32 & 0x3fu32) << 6)
                | (*$eptr.add(1) as u32 & 0x3fu32);
            $eptr = $eptr.add(2);
        } else if ($c & 0x08u32) == 0 {
            $c = (($c & 0x07u32) << 18)
                | ((*$eptr as u32 & 0x3fu32) << 12)
                | ((*$eptr.add(1) as u32 & 0x3fu32) << 6)
                | (*$eptr.add(2) as u32 & 0x3fu32);
            $eptr = $eptr.add(3);
        } else if ($c & 0x04u32) == 0 {
            $c = (($c & 0x03u32) << 24)
                | ((*$eptr as u32 & 0x3fu32) << 18)
                | ((*$eptr.add(1) as u32 & 0x3fu32) << 12)
                | ((*$eptr.add(2) as u32 & 0x3fu32) << 6)
                | (*$eptr.add(3) as u32 & 0x3fu32);
            $eptr = $eptr.add(4);
        } else {
            $c = (($c & 0x01u32) << 30)
                | ((*$eptr as u32 & 0x3fu32) << 24)
                | ((*$eptr.add(1) as u32 & 0x3fu32) << 18)
                | ((*$eptr.add(2) as u32 & 0x3fu32) << 12)
                | ((*$eptr.add(3) as u32 & 0x3fu32) << 6)
                | (*$eptr.add(4) as u32 & 0x3fu32);
            $eptr = $eptr.add(5);
        }
    }};
}

/// GETUTF8LEN(c, eptr, len): complete the character in `c`, adding to `len`.
#[macro_export]
macro_rules! GETUTF8LEN {
    ($c:expr, $eptr:expr, $len:expr) => {{
        if ($c & 0x20u32) == 0 {
            $c = (($c & 0x1fu32) << 6) | (*$eptr.add(1) as u32 & 0x3fu32);
            $len += 1;
        } else if ($c & 0x10u32) == 0 {
            $c = (($c & 0x0fu32) << 12)
                | ((*$eptr.add(1) as u32 & 0x3fu32) << 6)
                | (*$eptr.add(2) as u32 & 0x3fu32);
            $len += 2;
        } else if ($c & 0x08u32) == 0 {
            $c = (($c & 0x07u32) << 18)
                | ((*$eptr.add(1) as u32 & 0x3fu32) << 12)
                | ((*$eptr.add(2) as u32 & 0x3fu32) << 6)
                | (*$eptr.add(3) as u32 & 0x3fu32);
            $len += 3;
        } else if ($c & 0x04u32) == 0 {
            $c = (($c & 0x03u32) << 24)
                | ((*$eptr.add(1) as u32 & 0x3fu32) << 18)
                | ((*$eptr.add(2) as u32 & 0x3fu32) << 12)
                | ((*$eptr.add(3) as u32 & 0x3fu32) << 6)
                | (*$eptr.add(4) as u32 & 0x3fu32);
            $len += 4;
        } else {
            $c = (($c & 0x01u32) << 30)
                | ((*$eptr.add(1) as u32 & 0x3fu32) << 24)
                | ((*$eptr.add(2) as u32 & 0x3fu32) << 18)
                | ((*$eptr.add(3) as u32 & 0x3fu32) << 12)
                | ((*$eptr.add(4) as u32 & 0x3fu32) << 6)
                | (*$eptr.add(5) as u32 & 0x3fu32);
            $len += 5;
        }
    }};
}

/// GETCHAR(c, eptr): UTF mode known; does not advance eptr.
#[macro_export]
macro_rules! GETCHAR {
    ($c:expr, $eptr:expr) => {{
        $c = *$eptr as u32;
        if $c >= 0xc0u32 {
            GETUTF8!($c, $eptr);
        }
    }};
}

/// GETCHARTEST(c, eptr, utf): tests utf; does not advance eptr.
#[macro_export]
macro_rules! GETCHARTEST {
    ($c:expr, $eptr:expr, $utf:expr) => {{
        $c = *$eptr as u32;
        if $utf != 0 && $c >= 0xc0u32 {
            GETUTF8!($c, $eptr);
        }
    }};
}

/// GETCHARINC(c, eptr): UTF mode known; advances eptr.
#[macro_export]
macro_rules! GETCHARINC {
    ($c:expr, $eptr:expr) => {{
        $c = *$eptr as u32;
        $eptr = $eptr.add(1);
        if $c >= 0xc0u32 {
            GETUTF8INC!($c, $eptr);
        }
    }};
}

/// GETCHARINCTEST(c, eptr, utf): tests utf; advances eptr.
#[macro_export]
macro_rules! GETCHARINCTEST {
    ($c:expr, $eptr:expr, $utf:expr) => {{
        $c = *$eptr as u32;
        $eptr = $eptr.add(1);
        if $utf != 0 && $c >= 0xc0u32 {
            GETUTF8INC!($c, $eptr);
        }
    }};
}

/// GETCHARLEN(c, eptr, len): UTF mode known; does not advance eptr.
#[macro_export]
macro_rules! GETCHARLEN {
    ($c:expr, $eptr:expr, $len:expr) => {{
        $c = *$eptr as u32;
        if $c >= 0xc0u32 {
            GETUTF8LEN!($c, $eptr, $len);
        }
    }};
}

/// GETCHARLENTEST(c, eptr, len, utf): tests utf; does not advance eptr.
#[macro_export]
macro_rules! GETCHARLENTEST {
    ($c:expr, $eptr:expr, $len:expr, $utf:expr) => {{
        $c = *$eptr as u32;
        if $utf != 0 && $c >= 0xc0u32 {
            GETUTF8LEN!($c, $eptr, $len);
        }
    }};
}

/// BACKCHAR(eptr): move back to the start of a UTF-8 character.
#[macro_export]
macro_rules! BACKCHAR {
    ($eptr:expr) => {
        while (*$eptr & 0xc0u8) == 0x80u8 {
            $eptr = $eptr.offset(-1);
        }
    };
}

/// FORWARDCHAR(eptr)
#[macro_export]
macro_rules! FORWARDCHAR {
    ($eptr:expr) => {
        while (*$eptr & 0xc0u8) == 0x80u8 {
            $eptr = $eptr.add(1);
        }
    };
}

/// FORWARDCHARTEST(eptr, end)
#[macro_export]
macro_rules! FORWARDCHARTEST {
    ($eptr:expr, $end:expr) => {
        while $eptr < $end && (*$eptr & 0xc0u8) == 0x80u8 {
            $eptr = $eptr.add(1);
        }
    };
}

/// ACROSSCHAR(condition, eptr, action)
#[macro_export]
macro_rules! ACROSSCHAR {
    ($condition:expr, $eptr:expr, $action:stmt) => {
        while ($condition) && (*$eptr & 0xc0u8) == 0x80u8 {
            $action
        }
    };
}

/// PUTCHAR(c, p, utf): deposit a character, returning the number of code units.
#[macro_export]
macro_rules! PUTCHAR {
    ($c:expr, $p:expr, $utf:expr) => {
        if $utf != 0 && ($c as u32) > crate::consts::MAX_UTF_SINGLE_CU {
            crate::ord2utf::_pcre2_ord2utf_8($c as u32, $p)
        } else {
            *$p = $c as u8;
            1u32
        }
    };
}

/* ---------------- UCD access ---------------- */

/// GET_UCD(ch) -> *const ucd_record
#[macro_export]
macro_rules! GET_UCD {
    ($ch:expr) => {
        crate::ucd::_pcre2_ucd_records_8.as_ptr().add(
            crate::ucd::_pcre2_ucd_stage2_8[(crate::ucd::_pcre2_ucd_stage1_8
                [($ch as u32 as usize) / crate::consts::UCD_BLOCK_SIZE] as usize)
                * crate::consts::UCD_BLOCK_SIZE
                + ($ch as u32 as usize) % crate::consts::UCD_BLOCK_SIZE] as usize,
        )
    };
}

#[macro_export]
macro_rules! UCD_CHARTYPE {
    ($ch:expr) => {
        (*GET_UCD!($ch)).chartype as u32
    };
}

#[macro_export]
macro_rules! UCD_SCRIPT {
    ($ch:expr) => {
        (*GET_UCD!($ch)).script as u32
    };
}

#[macro_export]
macro_rules! UCD_CATEGORY {
    ($ch:expr) => {
        crate::tables::_pcre2_ucp_gentype_8[UCD_CHARTYPE!($ch) as usize]
    };
}

#[macro_export]
macro_rules! UCD_GRAPHBREAK {
    ($ch:expr) => {
        (*GET_UCD!($ch)).gbprop as u32
    };
}

#[macro_export]
macro_rules! UCD_CASESET {
    ($ch:expr) => {
        (*GET_UCD!($ch)).caseset as u32
    };
}

#[macro_export]
macro_rules! UCD_OTHERCASE {
    ($ch:expr) => {
        (($ch as u32 as i32) + ((*GET_UCD!($ch)).other_case as i32)) as u32
    };
}

#[macro_export]
macro_rules! UCD_SCRIPTX_PROP {
    ($prop:expr) => {
        ((*$prop).scriptx_bidiclass as u32 & crate::consts::UCD_SCRIPTX_MASK)
    };
}

#[macro_export]
macro_rules! UCD_BIDICLASS_PROP {
    ($prop:expr) => {
        ((*$prop).scriptx_bidiclass as u32 >> crate::consts::UCD_BIDICLASS_SHIFT)
    };
}

#[macro_export]
macro_rules! UCD_BPROPS_PROP {
    ($prop:expr) => {
        ((*$prop).bprops as u32 & crate::consts::UCD_BPROPS_MASK)
    };
}

#[macro_export]
macro_rules! UCD_SCRIPTX {
    ($ch:expr) => {
        UCD_SCRIPTX_PROP!(GET_UCD!($ch))
    };
}

#[macro_export]
macro_rules! UCD_BPROPS {
    ($ch:expr) => {
        UCD_BPROPS_PROP!(GET_UCD!($ch))
    };
}

#[macro_export]
macro_rules! UCD_BIDICLASS {
    ($ch:expr) => {
        UCD_BIDICLASS_PROP!(GET_UCD!($ch))
    };
}

#[macro_export]
macro_rules! UCD_ANY_I {
    ($ch:expr) => {
        (($ch as u32) | 0x20u32) == 0x69u32 || (($ch as u32) | 1u32) == 0x0131u32
    };
}

#[macro_export]
macro_rules! UCD_DOTTED_I {
    ($ch:expr) => {
        ($ch as u32) == 0x69u32 || ($ch as u32) == 0x0130u32
    };
}

#[macro_export]
macro_rules! UCD_FOLD_I_TURKISH {
    ($ch:expr) => {
        if ($ch as u32) == 0x0130u32 {
            0x69u32
        } else if ($ch as u32) == 0x49u32 {
            0x0131u32
        } else {
            $ch as u32
        }
    };
}

/* ---------------- Newline testing ---------------- */

/// The body of the IS_NEWLINE macro; the caller supplies the fields of its
/// own "NLBLOCK".
#[inline]
pub unsafe fn is_newline_block(
    p: crate::types::PCRE2_SPTR,
    nltype: u32,
    nllen: *mut u32,
    nl: *const u8,
    psend: crate::types::PCRE2_SPTR,
    utf: crate::types::BOOL,
) -> crate::types::BOOL {
    if nltype != crate::consts::NLTYPE_FIXED {
        (p < psend && crate::newline::_pcre2_is_newline_8(p, nltype, psend, nllen, utf) != 0)
            as crate::types::BOOL
    } else {
        (p <= psend.wrapping_sub(*nllen as usize)
            && *p == *nl
            && (*nllen == 1 || *p.add(1) == *nl.add(1))) as crate::types::BOOL
    }
}

/// The body of the WAS_NEWLINE macro.
#[inline]
pub unsafe fn was_newline_block(
    p: crate::types::PCRE2_SPTR,
    nltype: u32,
    nllen: *mut u32,
    nl: *const u8,
    psstart: crate::types::PCRE2_SPTR,
    utf: crate::types::BOOL,
) -> crate::types::BOOL {
    if nltype != crate::consts::NLTYPE_FIXED {
        (p > psstart && crate::newline::_pcre2_was_newline_8(p, nltype, psstart, nllen, utf) != 0)
            as crate::types::BOOL
    } else {
        (p >= psstart.wrapping_add(*nllen as usize)
            && *p.wrapping_sub(*nllen as usize) == *nl
            && (*nllen == 1 || *p.wrapping_sub(*nllen as usize).add(1) == *nl.add(1)))
            as crate::types::BOOL
    }
}

/* ---------------- Miscellaneous helpers ---------------- */

/// CU2BYTES / BYTES2CU are the identity in 8-bit mode.
#[macro_export]
macro_rules! CU2BYTES {
    ($x:expr) => {
        $x
    };
}

#[macro_export]
macro_rules! BYTES2CU {
    ($x:expr) => {
        $x
    };
}

/// CLIST_ALIGN_TO(base, align)
#[macro_export]
macro_rules! CLIST_ALIGN_TO {
    ($base:expr, $align:expr) => {
        (($base as usize + (($align as usize) - 1)) & !(($align as usize) - 1))
    };
}

/// GET_MAX_CHAR_VALUE(utf)
#[macro_export]
macro_rules! GET_MAX_CHAR_VALUE {
    ($utf:expr) => {
        if $utf != 0 {
            crate::consts::MAX_UTF_CODE_POINT
        } else {
            crate::consts::MAX_UCHAR_VALUE
        }
    };
}

/// SELECT_VALUE8(value8, value) - 8-bit mode selects the first.
#[macro_export]
macro_rules! SELECT_VALUE8 {
    ($value8:expr, $value:expr) => {
        $value8
    };
}

/// META_CODE / META_DATA / META_DIFF
#[macro_export]
macro_rules! META_CODE {
    ($x:expr) => {
        ($x & 0xffff0000u32)
    };
}

#[macro_export]
macro_rules! META_DATA {
    ($x:expr) => {
        ($x & 0x0000ffffu32)
    };
}

#[macro_export]
macro_rules! META_DIFF {
    ($x:expr, $y:expr) => {
        (($x).wrapping_sub($y) >> 16)
    };
}

/// PUTOFFSET(s,p): store a PCRE2_SIZE in the parsed pattern (64-bit world).
#[macro_export]
macro_rules! PUTOFFSET {
    ($s:expr, $p:expr) => {{
        *$p = (($s as u64) >> 32) as u32;
        $p = $p.add(1);
        *$p = (($s as u64) & 0xffffffff) as u32;
        $p = $p.add(1);
    }};
}

/// GETOFFSET(s,p)
#[macro_export]
macro_rules! GETOFFSET {
    ($s:expr, $p:expr) => {{
        $s = (((*$p.add(0) as u64) << 32) | (*$p.add(1) as u64)) as usize;
        $p = $p.add(2);
    }};
}

/// GETPLUSOFFSET(s,p)
#[macro_export]
macro_rules! GETPLUSOFFSET {
    ($s:expr, $p:expr) => {{
        $s = (((*$p.add(1) as u64) << 32) | (*$p.add(2) as u64)) as usize;
        $p = $p.add(2);
    }};
}

/// READPLUSOFFSET(s,p)
#[macro_export]
macro_rules! READPLUSOFFSET {
    ($s:expr, $p:expr) => {
        $s = (((*$p.add(1) as u64) << 32) | (*$p.add(2) as u64)) as usize
    };
}

/// SKIPOFFSET(p)
#[macro_export]
macro_rules! SKIPOFFSET {
    ($p:expr) => {
        $p = $p.add(2)
    };
}
