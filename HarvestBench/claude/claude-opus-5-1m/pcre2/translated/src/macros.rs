// Macros translated from pcre2_internal.h / pcre2_intmodedep.h / pcre2_compile.h.
// 8-bit code unit width, SUPPORT_UNICODE enabled, LINK_SIZE == 2.
//
// CONVENTIONS
// -----------
// * All pointer arguments are raw pointers (`*const u8` / `*mut u8` etc.), so all
//   of these macros must be used inside an `unsafe` block/fn.
// * Macros that assign to their first argument (like the C versions) work with any
//   place expression: a local, `(*ptr).field`, etc.
// * Where a C macro implicitly referenced a variable named `utf` (GETCHARTEST etc.)
//   the Rust macro takes it as an extra final argument, because Rust macros are
//   hygienic.
#![allow(unused_macros)]

// ---------------------------------------------------------------- GET / PUT

// GET(a,n): read a 2-byte (LINK_SIZE) big-endian value, returns u32
#[macro_export]
macro_rules! GET {
    ($a:expr, $n:expr) => {
        ((*($a).offset(($n) as isize) as u32) << 8) | (*($a).offset(($n) as isize + 1) as u32)
    };
}

// PUT(a,n,d): store a 2-byte (LINK_SIZE) big-endian value
#[macro_export]
macro_rules! PUT {
    ($a:expr, $n:expr, $d:expr) => {{
        let d__ = $d;
        *($a).offset(($n) as isize) = (d__ >> 8) as u8;
        *($a).offset(($n) as isize + 1) = (d__ & 255) as u8;
    }};
}

// GET2(a,n): read a 2-byte (IMM2_SIZE) big-endian value, returns u32
#[macro_export]
macro_rules! GET2 {
    ($a:expr, $n:expr) => {
        ((*($a).offset(($n) as isize) as u32) << 8) | (*($a).offset(($n) as isize + 1) as u32)
    };
}

// PUT2(a,n,d)
#[macro_export]
macro_rules! PUT2 {
    ($a:expr, $n:expr, $d:expr) => {{
        let d__ = $d;
        *($a).offset(($n) as isize) = (d__ >> 8) as u8;
        *($a).offset(($n) as isize + 1) = (d__ & 255) as u8;
    }};
}

// PUTINC(a,n,d): PUT then advance a by LINK_SIZE
#[macro_export]
macro_rules! PUTINC {
    ($a:expr, $n:expr, $d:expr) => {{
        PUT!($a, $n, $d);
        $a = ($a).add($crate::internal::LINK_SIZE);
    }};
}

// PUT2INC(a,n,d): PUT2 then advance a by IMM2_SIZE
#[macro_export]
macro_rules! PUT2INC {
    ($a:expr, $n:expr, $d:expr) => {{
        PUT2!($a, $n, $d);
        $a = ($a).add($crate::internal::IMM2_SIZE);
    }};
}

// ------------------------------------------------------------ UTF-8 decoding

// HASUTF8EXTRALEN(c)
#[macro_export]
macro_rules! HASUTF8EXTRALEN {
    ($c:expr) => {
        ($c) >= 0xc0
    };
}

// HAS_EXTRALEN(c)
#[macro_export]
macro_rules! HAS_EXTRALEN {
    ($c:expr) => {
        ($c) >= 0xc0
    };
}

// GET_EXTRALEN(c) - number of additional code units, returns u32
#[macro_export]
macro_rules! GET_EXTRALEN {
    ($c:expr) => {
        $crate::pcre2_tables::_pcre2_utf8_table4[(($c) & 0x3f) as usize] as u32
    };
}

// NOT_FIRSTCU(c)
#[macro_export]
macro_rules! NOT_FIRSTCU {
    ($c:expr) => {
        (($c) & 0xc0) == 0x80
    };
}

// GETUTF8(c, eptr): complete decoding of a character whose first code unit is
// already in c; does not advance eptr.
#[macro_export]
macro_rules! GETUTF8 {
    ($c:expr, $eptr:expr) => {{
        if ($c & 0x20u32) == 0 {
            $c = (($c & 0x1fu32) << 6) | (*($eptr).offset(1) as u32 & 0x3fu32);
        } else if ($c & 0x10u32) == 0 {
            $c = (($c & 0x0fu32) << 12)
                | ((*($eptr).offset(1) as u32 & 0x3fu32) << 6)
                | (*($eptr).offset(2) as u32 & 0x3fu32);
        } else if ($c & 0x08u32) == 0 {
            $c = (($c & 0x07u32) << 18)
                | ((*($eptr).offset(1) as u32 & 0x3fu32) << 12)
                | ((*($eptr).offset(2) as u32 & 0x3fu32) << 6)
                | (*($eptr).offset(3) as u32 & 0x3fu32);
        } else if ($c & 0x04u32) == 0 {
            $c = (($c & 0x03u32) << 24)
                | ((*($eptr).offset(1) as u32 & 0x3fu32) << 18)
                | ((*($eptr).offset(2) as u32 & 0x3fu32) << 12)
                | ((*($eptr).offset(3) as u32 & 0x3fu32) << 6)
                | (*($eptr).offset(4) as u32 & 0x3fu32);
        } else {
            $c = (($c & 0x01u32) << 30)
                | ((*($eptr).offset(1) as u32 & 0x3fu32) << 24)
                | ((*($eptr).offset(2) as u32 & 0x3fu32) << 18)
                | ((*($eptr).offset(3) as u32 & 0x3fu32) << 12)
                | ((*($eptr).offset(4) as u32 & 0x3fu32) << 6)
                | (*($eptr).offset(5) as u32 & 0x3fu32);
        }
    }};
}

// GETUTF8INC(c, eptr): as above but advances eptr past the trailing code units.
// NOTE: like the C macro, eptr must already have been advanced past the first
// code unit.
#[macro_export]
macro_rules! GETUTF8INC {
    ($c:expr, $eptr:expr) => {{
        if ($c & 0x20u32) == 0 {
            $c = (($c & 0x1fu32) << 6) | (*$eptr as u32 & 0x3fu32);
            $eptr = ($eptr).add(1);
        } else if ($c & 0x10u32) == 0 {
            $c = (($c & 0x0fu32) << 12)
                | ((*$eptr as u32 & 0x3fu32) << 6)
                | (*($eptr).offset(1) as u32 & 0x3fu32);
            $eptr = ($eptr).add(2);
        } else if ($c & 0x08u32) == 0 {
            $c = (($c & 0x07u32) << 18)
                | ((*$eptr as u32 & 0x3fu32) << 12)
                | ((*($eptr).offset(1) as u32 & 0x3fu32) << 6)
                | (*($eptr).offset(2) as u32 & 0x3fu32);
            $eptr = ($eptr).add(3);
        } else if ($c & 0x04u32) == 0 {
            $c = (($c & 0x03u32) << 24)
                | ((*$eptr as u32 & 0x3fu32) << 18)
                | ((*($eptr).offset(1) as u32 & 0x3fu32) << 12)
                | ((*($eptr).offset(2) as u32 & 0x3fu32) << 6)
                | (*($eptr).offset(3) as u32 & 0x3fu32);
            $eptr = ($eptr).add(4);
        } else {
            $c = (($c & 0x01u32) << 30)
                | ((*$eptr as u32 & 0x3fu32) << 24)
                | ((*($eptr).offset(1) as u32 & 0x3fu32) << 18)
                | ((*($eptr).offset(2) as u32 & 0x3fu32) << 12)
                | ((*($eptr).offset(3) as u32 & 0x3fu32) << 6)
                | (*($eptr).offset(4) as u32 & 0x3fu32);
            $eptr = ($eptr).add(5);
        }
    }};
}

// GETUTF8LEN(c, eptr, len): does not advance eptr, adds the number of extra
// code units to len.
#[macro_export]
macro_rules! GETUTF8LEN {
    ($c:expr, $eptr:expr, $len:expr) => {{
        if ($c & 0x20u32) == 0 {
            $c = (($c & 0x1fu32) << 6) | (*($eptr).offset(1) as u32 & 0x3fu32);
            $len += 1;
        } else if ($c & 0x10u32) == 0 {
            $c = (($c & 0x0fu32) << 12)
                | ((*($eptr).offset(1) as u32 & 0x3fu32) << 6)
                | (*($eptr).offset(2) as u32 & 0x3fu32);
            $len += 2;
        } else if ($c & 0x08u32) == 0 {
            $c = (($c & 0x07u32) << 18)
                | ((*($eptr).offset(1) as u32 & 0x3fu32) << 12)
                | ((*($eptr).offset(2) as u32 & 0x3fu32) << 6)
                | (*($eptr).offset(3) as u32 & 0x3fu32);
            $len += 3;
        } else if ($c & 0x04u32) == 0 {
            $c = (($c & 0x03u32) << 24)
                | ((*($eptr).offset(1) as u32 & 0x3fu32) << 18)
                | ((*($eptr).offset(2) as u32 & 0x3fu32) << 12)
                | ((*($eptr).offset(3) as u32 & 0x3fu32) << 6)
                | (*($eptr).offset(4) as u32 & 0x3fu32);
            $len += 4;
        } else {
            $c = (($c & 0x01u32) << 30)
                | ((*($eptr).offset(1) as u32 & 0x3fu32) << 24)
                | ((*($eptr).offset(2) as u32 & 0x3fu32) << 18)
                | ((*($eptr).offset(3) as u32 & 0x3fu32) << 12)
                | ((*($eptr).offset(4) as u32 & 0x3fu32) << 6)
                | (*($eptr).offset(5) as u32 & 0x3fu32);
            $len += 5;
        }
    }};
}

// GETCHAR(c, eptr): c must be a u32 place; does not advance eptr
#[macro_export]
macro_rules! GETCHAR {
    ($c:expr, $eptr:expr) => {{
        $c = *($eptr) as u32;
        if $c >= 0xc0u32 {
            GETUTF8!($c, $eptr);
        }
    }};
}

// GETCHARTEST(c, eptr, utf)
#[macro_export]
macro_rules! GETCHARTEST {
    ($c:expr, $eptr:expr, $utf:expr) => {{
        $c = *($eptr) as u32;
        if ($utf) != 0 && $c >= 0xc0u32 {
            GETUTF8!($c, $eptr);
        }
    }};
}

// GETCHARINC(c, eptr)
#[macro_export]
macro_rules! GETCHARINC {
    ($c:expr, $eptr:expr) => {{
        $c = *($eptr) as u32;
        $eptr = ($eptr).add(1);
        if $c >= 0xc0u32 {
            GETUTF8INC!($c, $eptr);
        }
    }};
}

// GETCHARINCTEST(c, eptr, utf)
#[macro_export]
macro_rules! GETCHARINCTEST {
    ($c:expr, $eptr:expr, $utf:expr) => {{
        $c = *($eptr) as u32;
        $eptr = ($eptr).add(1);
        if ($utf) != 0 && $c >= 0xc0u32 {
            GETUTF8INC!($c, $eptr);
        }
    }};
}

// GETCHARLEN(c, eptr, len)
#[macro_export]
macro_rules! GETCHARLEN {
    ($c:expr, $eptr:expr, $len:expr) => {{
        $c = *($eptr) as u32;
        if $c >= 0xc0u32 {
            GETUTF8LEN!($c, $eptr, $len);
        }
    }};
}

// GETCHARLENTEST(c, eptr, len, utf)
#[macro_export]
macro_rules! GETCHARLENTEST {
    ($c:expr, $eptr:expr, $len:expr, $utf:expr) => {{
        $c = *($eptr) as u32;
        if ($utf) != 0 && $c >= 0xc0u32 {
            GETUTF8LEN!($c, $eptr, $len);
        }
    }};
}

// BACKCHAR(eptr): move back to the start of a UTF-8 character
#[macro_export]
macro_rules! BACKCHAR {
    ($eptr:expr) => {
        while (*($eptr) & 0xc0u8) == 0x80u8 {
            $eptr = ($eptr).sub(1);
        }
    };
}

// FORWARDCHAR(eptr)
#[macro_export]
macro_rules! FORWARDCHAR {
    ($eptr:expr) => {
        while (*($eptr) & 0xc0u8) == 0x80u8 {
            $eptr = ($eptr).add(1);
        }
    };
}

// FORWARDCHARTEST(eptr, end)
#[macro_export]
macro_rules! FORWARDCHARTEST {
    ($eptr:expr, $end:expr) => {
        while ($eptr) < ($end) && (*($eptr) & 0xc0u8) == 0x80u8 {
            $eptr = ($eptr).add(1);
        }
    };
}

// ACROSSCHAR(condition, eptr, action)
#[macro_export]
macro_rules! ACROSSCHAR {
    ($cond:expr, $eptr:expr, $action:expr) => {
        while ($cond) && (*($eptr) & 0xc0u8) == 0x80u8 {
            $action;
        }
    };
}

// PUTCHAR(c, p, utf) -> number of code units written (u32)
#[macro_export]
macro_rules! PUTCHAR {
    ($c:expr, $p:expr, $utf:expr) => {
        if ($utf) != 0 && $c > $crate::internal::MAX_UTF_SINGLE_CU {
            $crate::internal::_pcre2_ord2utf_8($c, $p)
        } else {
            *($p) = $c as u8;
            1u32
        }
    };
}

// ---------------------------------------------------------------- table access

// MAX_255(c) -> BOOL (always TRUE in 8-bit mode)
#[macro_export]
macro_rules! MAX_255 {
    ($c:expr) => {
        $crate::internal::TRUE
    };
}

// CHMAX_255(c) -> BOOL
#[macro_export]
macro_rules! CHMAX_255 {
    ($c:expr) => {
        ((($c) <= 255u32) as $crate::internal::BOOL)
    };
}

// TABLE_GET(c, table, default) -> u8 (8-bit mode ignores the default)
#[macro_export]
macro_rules! TABLE_GET {
    ($c:expr, $table:expr, $default:expr) => {
        *($table).offset(($c) as isize)
    };
}

// ---------------------------------------------------------------- newlines

// IS_NEWLINE(p): blk is a *mut/&mut struct with nltype/nllen/nl fields,
// psend is the end-of-subject pointer expression.
#[macro_export]
macro_rules! IS_NEWLINE {
    ($p:expr, $blk:expr, $psend:expr, $utf:expr) => {
        if (*$blk).nltype != $crate::internal::NLTYPE_FIXED {
            ($p) < ($psend)
                && $crate::internal::_pcre2_is_newline_8(
                    $p,
                    (*$blk).nltype,
                    $psend,
                    &mut (*$blk).nllen,
                    $utf,
                ) != 0
        } else {
            ($p) <= ($psend).offset(-((*$blk).nllen as isize))
                && *($p) == (*$blk).nl[0]
                && ((*$blk).nllen == 1 || *($p).offset(1) == (*$blk).nl[1])
        }
    };
}

// WAS_NEWLINE(p)
#[macro_export]
macro_rules! WAS_NEWLINE {
    ($p:expr, $blk:expr, $psstart:expr, $utf:expr) => {
        if (*$blk).nltype != $crate::internal::NLTYPE_FIXED {
            ($p) > ($psstart)
                && $crate::internal::_pcre2_was_newline_8(
                    $p,
                    (*$blk).nltype,
                    $psstart,
                    &mut (*$blk).nllen,
                    $utf,
                ) != 0
        } else {
            ($p) >= ($psstart).offset((*$blk).nllen as isize)
                && *($p).offset(-((*$blk).nllen as isize)) == (*$blk).nl[0]
                && ((*$blk).nllen == 1
                    || *($p).offset(-((*$blk).nllen as isize) + 1) == (*$blk).nl[1])
        }
    };
}

// ---------------------------------------------------------------- bit maps

// MAPBIT(map,n) -> u32 (non-zero if set); map is *const u32
#[macro_export]
macro_rules! MAPBIT {
    ($map:expr, $n:expr) => {
        (*($map).offset((($n) / 32) as isize) & (1u32 << (($n) % 32)))
    };
}

// MAPSET(map,n); map is *mut u32
#[macro_export]
macro_rules! MAPSET {
    ($map:expr, $n:expr) => {
        *($map).offset((($n) / 32) as isize) |= 1u32 << (($n) % 32)
    };
}

// SETBIT(a,b): a is *mut u8 (class bitmap)
#[macro_export]
macro_rules! SETBIT {
    ($a:expr, $b:expr) => {
        *($a).offset((($b) >> 3) as isize) |= (1u32 << (($b) & 0x7)) as u8
    };
}

// ---------------------------------------------------------------- parsed pattern

// META_CODE(x), META_DATA(x), META_DIFF(x,y)
#[macro_export]
macro_rules! META_CODE {
    ($x:expr) => {
        ($x) & 0xffff0000u32
    };
}
#[macro_export]
macro_rules! META_DATA {
    ($x:expr) => {
        ($x) & 0x0000ffffu32
    };
}
#[macro_export]
macro_rules! META_DIFF {
    ($x:expr, $y:expr) => {
        (($x) - ($y)) >> 16
    };
}

// PUTOFFSET(s,p): store a PCRE2_SIZE into two uint32_t elements, advancing p
#[macro_export]
macro_rules! PUTOFFSET {
    ($s:expr, $p:expr) => {{
        *$p = (($s) >> 32) as u32;
        $p = ($p).add(1);
        *$p = (($s) & 0xffffffff) as u32;
        $p = ($p).add(1);
    }};
}

// GETOFFSET(s,p): read a PCRE2_SIZE from two uint32_t elements, advancing p
#[macro_export]
macro_rules! GETOFFSET {
    ($s:expr, $p:expr) => {{
        $s = ((*($p).offset(0) as $crate::internal::PCRE2_SIZE) << 32)
            | (*($p).offset(1) as $crate::internal::PCRE2_SIZE);
        $p = ($p).add(2);
    }};
}

// GETPLUSOFFSET(s,p): read from p[1],p[2], advancing p by 2
#[macro_export]
macro_rules! GETPLUSOFFSET {
    ($s:expr, $p:expr) => {{
        $s = ((*($p).offset(1) as $crate::internal::PCRE2_SIZE) << 32)
            | (*($p).offset(2) as $crate::internal::PCRE2_SIZE);
        $p = ($p).add(2);
    }};
}

// READPLUSOFFSET(s,p): read from p[1],p[2] without advancing
#[macro_export]
macro_rules! READPLUSOFFSET {
    ($s:expr, $p:expr) => {
        $s = ((*($p).offset(1) as $crate::internal::PCRE2_SIZE) << 32)
            | (*($p).offset(2) as $crate::internal::PCRE2_SIZE)
    };
}

// SKIPOFFSET(p)
#[macro_export]
macro_rules! SKIPOFFSET {
    ($p:expr) => {
        $p = ($p).add(2)
    };
}

// ---------------------------------------------------------------- misc

// CU2BYTES / BYTES2CU are the identity in 8-bit mode
#[macro_export]
macro_rules! CU2BYTES {
    ($x:expr) => {
        ($x)
    };
}
#[macro_export]
macro_rules! BYTES2CU {
    ($x:expr) => {
        ($x)
    };
}

// CLIST_ALIGN_TO(base, align)
#[macro_export]
macro_rules! CLIST_ALIGN_TO {
    ($base:expr, $align:expr) => {
        (($base) + (($align) as usize - 1)) & !(($align) as usize - 1)
    };
}

// GET_MAX_CHAR_VALUE(utf)
#[macro_export]
macro_rules! GET_MAX_CHAR_VALUE {
    ($utf:expr) => {
        if ($utf) != 0 {
            $crate::internal::MAX_UTF_CODE_POINT
        } else {
            $crate::internal::MAX_UCHAR_VALUE
        }
    };
}

// SELECT_VALUE8(value8, value) - 8-bit mode always selects value8
#[macro_export]
macro_rules! SELECT_VALUE8 {
    ($value8:expr, $value:expr) => {
        $value8
    };
}
