mod common;
use common::*;

#[test]
fn loads_both_libraries() {
    let (c, r) = both();
    assert_eq!(c.tag, "C");
    assert_eq!(r.tag, "RUST");
    println!("C   = {:?}", c.path);
    println!("RUST= {:?}", r.path);
}

#[test]
fn compile_and_match_agree() {
    let (c, r) = both();
    unsafe {
        for pat in [
            &b"a(b|c)d"[..],
            &b"^\\d+$"[..],
            &b"(?i)HeLLo\\s+world"[..],
            &b"(?<n>x)(?&n)"[..],
            &b"[[:alpha:]]{2,5}"[..],
            &b"\\p{Greek}+"[..],
        ] {
            let a = c.compile_probe(pat, pat.len(), 0, std::ptr::null_mut());
            let b = r.compile_probe(pat, pat.len(), 0, std::ptr::null_mut());
            assert_eq!(a.ok, b.ok, "pattern {:?}", String::from_utf8_lossy(pat));
            assert_eq!(a.errorcode, b.errorcode);
            assert_eq!(a.erroroffset, b.erroroffset);
            assert_eq!(a.info, b.info, "info for {:?}", String::from_utf8_lossy(pat));
            assert_eq!(
                a.image,
                b.image,
                "serialized image differs for {:?}",
                String::from_utf8_lossy(pat)
            );
        }
    }
}

#[test]
fn leaf_functions_agree() {
    let (c, r) = both();
    unsafe {
        let s = b"hello\0";
        assert_eq!((c._pcre2_strlen_8)(s.as_ptr()), (r._pcre2_strlen_8)(s.as_ptr()));
        for cp in [0u32, 0x41, 0x7f, 0x80, 0x7ff, 0x800, 0xffff, 0x10000, 0x10ffff] {
            let mut b1 = [0u8; 8];
            let mut b2 = [0u8; 8];
            let n1 = (c._pcre2_ord2utf_8)(cp, b1.as_mut_ptr());
            let n2 = (r._pcre2_ord2utf_8)(cp, b2.as_mut_ptr());
            assert_eq!((n1, b1), (n2, b2), "ord2utf {:#x}", cp);
        }
    }
}
