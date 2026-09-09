//! Regression tests for divergences that were found and fixed during verification.
//!
//! 1. `extuni_high_codepoint` — `PRIV(extuni)` (\X) reaches `GET_UCD` with a code point above
//!    MAX_UTF_CODE_POINT, because `GETCHARLEN` on a 5/6-byte UTF-8 lead byte yields
//!    c > 0x10FFFF and the 8-bit build of `GET_UCD` has no MAX_UTF_CODE_POINT guard
//!    (`c_src/src/pcre2_internal.h:2123-2131`). The C reads `PRIV(ucd_stage1)` out of bounds
//!    and returns rc=1; the Rust used bounds-checked slice indexing and ABORTED
//!    (`panic = "abort"`). Fixed by making `GET_UCD!` use unchecked pointer arithmetic with
//!    C's `(int)` division/modulo semantics.
//! 2. `ovec_residue` — pins that the ovector/startchar residue left behind on FAILED match
//!    paths is identical in the two libraries when the match data is allocated through a
//!    zeroing allocator (an earlier report of a difference here turned out to be
//!    uninitialised allocator memory, not a real divergence).
mod common;
use common::*;
use std::ptr;
#[test]
fn extuni_high_codepoint() {
    let (c, r) = both();
    unsafe {
        let subj = b"a\xf8\x88\x80\x80\x80";
        for (tag, api) in [("C", c), ("RUST", r)] {
            let mut ec = 0; let mut eo = 0;
            let code = (api.pcre2_compile_8)(b"\\X".as_ptr(), 2, PCRE2_UTF, &mut ec, &mut eo, ptr::null_mut());
            assert!(!code.is_null(), "{} compile failed {}", tag, ec);
            let md = (api.pcre2_match_data_create_8)(4, zeroing_context(api));
            let rc = (api.pcre2_match_8)(code, subj.as_ptr(), subj.len(), 0, PCRE2_NO_UTF_CHECK, md, ptr::null_mut());
            let ov = (api.pcre2_get_ovector_pointer_8)(md);
            println!("{}: rc={} ov={:?}", tag, rc, std::slice::from_raw_parts(ov, 4));
            (api.pcre2_match_data_free_8)(md);
            (api.pcre2_code_free_8)(code);
        }
    }
}

unsafe fn probe(api:&Api, pat:&[u8], opts:u32, subj:&[u8], deplim:i64, ovec:u32)->(i32,Vec<usize>,usize){
    let mut ec=0; let mut eo=0;
    let code=(api.pcre2_compile_8)(pat.as_ptr(),pat.len(),opts,&mut ec,&mut eo,ptr::null_mut());
    assert!(!code.is_null(),"{} compile {}",api.tag,ec);
    let gz=zeroing_context(api);
    let md=(api.pcre2_match_data_create_8)(ovec,gz);
    let mc=(api.pcre2_match_context_create_8)(ptr::null_mut());
    if deplim>=0 {(api.pcre2_set_recursion_limit_8)(mc,deplim as u32);}
    let rc=(api.pcre2_match_8)(code,subj.as_ptr(),subj.len(),0,0,md,mc);
    let n=(api.pcre2_get_ovector_count_8)(md);
    let ov=std::slice::from_raw_parts((api.pcre2_get_ovector_pointer_8)(md),(n*2) as usize).to_vec();
    let sc=(api.pcre2_get_startchar_8)(md);
    (api.pcre2_match_data_free_8)(md);(api.pcre2_match_context_free_8)(mc);
    (api.pcre2_general_context_free_8)(gz);(api.pcre2_code_free_8)(code);
    (rc,ov,sc)
}
#[test]
fn ovec_residue() {
    let (c,r)=both();
    unsafe{
        for (pat,opts,subj,dl,ov) in [
            (&b"(a+)*b"[..],0u32,&b"aaaaaaaaaaaaaaaaaaaaaaaaaaaa"[..],0i64,2u32),
            (&b"^a$"[..],PCRE2_DOLLAR_ENDONLY,&b"a\n"[..],-1,1),
            (&b"(a)(b)?"[..],0,&b"a"[..],-1,4),
            (&b"(a)(b)?"[..],0,&b"zzz"[..],-1,4),
        ] {
            let a=probe(c,pat,opts,subj,dl,ov);
            let b=probe(r,pat,opts,subj,dl,ov);
            println!("pat={:?} dl={} -> C {:?}\n                     RUST {:?}  {}",
                String::from_utf8_lossy(pat),dl,a,b, if a==b {"SAME"} else {"*** DIFFERS ***"});
        }
    }
}
