// Temporary exploration probe for the phase-B dfa_match / substitute sign-off.
// Deleted once tests/phase_b_cfg_dfa_subst.rs is final.
mod common;
use common::*;
use std::ffi::{c_int, c_void};
use std::ptr;

#[repr(C)]
struct MdHead {
    memctl_malloc: *mut c_void,
    memctl_free: *mut c_void,
    memctl_data: *mut c_void,
    code: *const c_void,
    subject: Sptr,
    mark: Sptr,
    heapframes: *mut c_void,
    heapframes_size: Sz,
    subject_length: Sz,
    start_offset: Sz,
    leftchar: Sz,
    rightchar: Sz,
    startchar: Sz,
    matchedby: u8,
    flags: u8,
    oveccount: u16,
    options: u32,
    rc: c_int,
}

unsafe fn compile1(api: &Api, pat: &[u8], opts: u32) -> Ptr {
    let (mut ec, mut eo) = (0 as c_int, 0usize);
    let c = (api.compile)(pat.as_ptr(), pat.len(), opts, &mut ec, &mut eo, ptr::null_mut());
    assert!(!c.is_null(), "compile {} failed ec={ec} eo={eo}", show(pat));
    c
}

#[test]
fn probe_md_layout() {
    let p = pair();
    unsafe {
        for api in [&p.c, &p.r] {
            let code = compile1(api, b"(a)(b)?", 0);
            let md = (api.match_data_create)(7, ptr::null_mut());
            let subj = b"xxab";
            let rc = (api.do_match)(code, subj.as_ptr(), 4, 0, 0, md, ptr::null_mut());
            let h = &*(md as *const MdHead);
            println!(
                "[{}] rc={rc} h.rc={} oveccount={} startchar={} matchedby={} flags={} \
                 leftchar={} rightchar={} sublen={} start_offset={} options={:#x} subj_eq={}",
                api.name, h.rc, h.oveccount, h.startchar, h.matchedby, h.flags,
                h.leftchar, h.rightchar, h.subject_length, h.start_offset, h.options,
                h.subject == subj.as_ptr()
            );
            assert_eq!(h.rc, rc);
            assert_eq!(h.oveccount as u32, (api.get_ovector_count)(md));
            assert_eq!(h.startchar, (api.get_startchar)(md));
            assert_eq!(h.code as usize, code as usize);
            // now DFA
            let mut ws = vec![0 as c_int; 1000];
            let rc2 = (api.dfa_match)(
                code, subj.as_ptr(), 4, 0, 0, md, ptr::null_mut(), ws.as_mut_ptr(), 1000,
            );
            let h = &*(md as *const MdHead);
            println!(
                "[{}] dfa rc={rc2} matchedby={} leftchar={} rightchar={} startchar={}",
                api.name, h.matchedby, h.leftchar, h.rightchar, h.startchar
            );
            assert_eq!(h.matchedby, 1);
            (api.match_data_free)(md);
            (api.code_free)(code);
        }
    }
}

#[test]
fn probe_restart_workspace() {
    let p = pair();
    unsafe {
        for api in [&p.c, &p.r] {
            let code = compile1(api, b"abcd", 0);
            for wsn in [20usize, 32, 44] {
                let mut ws = vec![0 as c_int; wsn];
                let md = (api.match_data_create)(4, ptr::null_mut());
                let r1 = (api.dfa_match)(
                    code, b"ab".as_ptr(), 2, 0, PCRE2_PARTIAL_SOFT, md, ptr::null_mut(),
                    ws.as_mut_ptr(), wsn,
                );
                println!("[{}] ws={wsn} partial rc={r1} ws0={} ws1={}", api.name, ws[0], ws[1]);
                let bound = ((wsn - 2) / 3) as c_int;
                for w1 in [0 as c_int, 1, bound, bound + 1] {
                    let mut ws2 = ws.clone();
                    ws2[1] = w1;
                    let r2 = (api.dfa_match)(
                        code, b"cd".as_ptr(), 2, 0, PCRE2_DFA_RESTART, md, ptr::null_mut(),
                        ws2.as_mut_ptr(), wsn,
                    );
                    println!("   [{}] restart ws1={w1} (bound={bound}) rc={r2}", api.name);
                }
                for w0 in [0 as c_int, 1, 2, -1] {
                    let mut ws2 = ws.clone();
                    ws2[0] = w0;
                    let r2 = (api.dfa_match)(
                        code, b"cd".as_ptr(), 2, 0, PCRE2_DFA_RESTART, md, ptr::null_mut(),
                        ws2.as_mut_ptr(), wsn,
                    );
                    println!("   [{}] restart ws0={w0} rc={r2}", api.name);
                }
                (api.match_data_free)(md);
            }
            (api.code_free)(code);
        }
    }
}

#[test]
fn probe_turkish_subst() {
    let p = pair();
    unsafe {
        for api in [&p.c, &p.r] {
            let cc = (api.compile_context_create)(ptr::null_mut());
            (api.set_compile_extra_options)(cc, PCRE2_EXTRA_TURKISH_CASING);
            let pat = b"(\\w+)";
            let (mut ec, mut eo) = (0 as c_int, 0usize);
            let code = (api.compile)(
                pat.as_ptr(), pat.len(), PCRE2_UTF | PCRE2_UCP | PCRE2_CASELESS,
                &mut ec, &mut eo, cc,
            );
            assert!(!code.is_null(), "turkish compile err {ec}");
            for subj in ["i", "I", "\u{130}", "\u{131}", "ii"] {
                for rep in ["\\U$1\\E", "\\L$1\\E", "\\u$1", "\\l$1"] {
                    let sb = subj.as_bytes();
                    let rb = rep.as_bytes();
                    let mut out = vec![0xEEu8; 64];
                    let mut bl = 48usize;
                    let rc = (api.substitute)(
                        code, sb.as_ptr(), sb.len(), 0, PCRE2_SUBSTITUTE_EXTENDED,
                        ptr::null_mut(), ptr::null_mut(), rb.as_ptr(), rb.len(),
                        out.as_mut_ptr(), &mut bl,
                    );
                    println!(
                        "[{}] turkish subj={} rep={} rc={rc} out={}",
                        api.name, show(sb), rep, show(&out[..bl.min(48)])
                    );
                }
            }
            (api.code_free)(code);
            (api.compile_context_free)(cc);
        }
    }
}

static mut LOG: Vec<String> = Vec::new();

#[repr(C)]
struct CalloutBlock {
    version: u32,
    callout_number: u32,
    capture_top: u32,
    capture_last: u32,
    offset_vector: *const Sz,
    mark: Sptr,
    subject: Sptr,
    subject_length: Sz,
    start_match: Sz,
    current_position: Sz,
    pattern_position: Sz,
    next_item_length: Sz,
    callout_string_offset: Sz,
    callout_string_length: Sz,
    callout_string: Sptr,
    callout_flags: u32,
}

static mut CALLOUT_RET: c_int = 0;

unsafe extern "C" fn cb(blk: *mut c_void, _d: *mut c_void) -> c_int {
    let b = &*(blk as *const CalloutBlock);
    let log = &mut *ptr::addr_of_mut!(LOG);
    log.push(format!(
        "v={} n={} ct={} cl={} sm={} cp={} pp={} nil={} mark_null={}",
        b.version, b.callout_number, b.capture_top, b.capture_last, b.start_match,
        b.current_position, b.pattern_position, b.next_item_length, b.mark.is_null()
    ));
    *ptr::addr_of_mut!(CALLOUT_RET)
}

#[test]
fn probe_dfa_callout() {
    let p = pair();
    unsafe {
        for api in [&p.c, &p.r] {
            let code = compile1(api, b"a(?C1)b|a(?C2)c", 0);
            let mc = (api.match_context_create)(ptr::null_mut());
            (api.set_callout)(mc, Some(cb), ptr::null_mut());
            for r in [0 as c_int, 1, -1, -2] {
                CALLOUT_RET = r;
                LOG.clear();
                let md = (api.match_data_create)(4, ptr::null_mut());
                let mut ws = vec![0 as c_int; 1000];
                let rc = (api.dfa_match)(
                    code, b"zabc".as_ptr(), 4, 0, 0, md, mc, ws.as_mut_ptr(), 1000,
                );
                println!("[{}] dfa callout ret={r} rc={rc} log={:?}", api.name, LOG);
                (api.match_data_free)(md);
            }
            (api.match_context_free)(mc);
            (api.code_free)(code);
        }
    }
}

#[test]
fn probe_dfa_heaplimit() {
    let p = pair();
    unsafe {
        for api in [&p.c, &p.r] {
            // 8 nested assertions -> exceeds the 7676-int base RWS block
            let pat = b"(?=(?=(?=(?=(?=(?=(?=(?=a)))))))) ";
            let code = compile1(api, &pat[..pat.len() - 1], 0);
            let mc = (api.match_context_create)(ptr::null_mut());
            for hl in [0u32, 1, 3, 4, 5, 60, 1000] {
                (api.set_heap_limit)(mc, hl);
                let md = (api.match_data_create)(4, ptr::null_mut());
                let mut ws = vec![0 as c_int; 1000];
                let rc = (api.dfa_match)(
                    code, b"a".as_ptr(), 1, 0, 0, md, mc, ws.as_mut_ptr(), 1000,
                );
                println!("[{}] 8-assert heap_limit={hl} rc={rc}", api.name);
                (api.match_data_free)(md);
            }
            // recursion needs >= 12 KiB
            let code2 = compile1(api, b"\\((?:[^()]++|(?R))*\\)", 0);
            for hl in [0u32, 4, 11, 12, 13, 1000] {
                (api.set_heap_limit)(mc, hl);
                let md = (api.match_data_create)(4, ptr::null_mut());
                let mut ws = vec![0 as c_int; 1000];
                let s = b"((((((((((a))))))))))";
                let rc = (api.dfa_match)(
                    code2, s.as_ptr(), s.len(), 0, 0, md, mc, ws.as_mut_ptr(), 1000,
                );
                println!("[{}] recursion heap_limit={hl} rc={rc}", api.name);
                (api.match_data_free)(md);
            }
            (api.match_context_free)(mc);
            (api.code_free)(code);
            (api.code_free)(code2);
        }
    }
}

#[test]
fn probe_subst_case_callout_overflow() {
    let p = pair();
    unsafe extern "C" fn inflate(
        input: Sptr, inlen: Sz, output: *mut u8, outlen: Sz, to_case: c_int, _d: *mut c_void,
    ) -> Sz {
        // grow by 4x -- far more than (len>>3)+10 for long inputs
        let need = inlen * 4;
        if need > outlen {
            return need;
        }
        for i in 0..inlen {
            let c = *input.add(i);
            for k in 0..4 {
                *output.add(i * 4 + k) = if to_case == 1 {
                    c.to_ascii_lowercase()
                } else {
                    c.to_ascii_uppercase()
                };
            }
        }
        need
    }
    unsafe {
        for api in [&p.c, &p.r] {
            let code = compile1(api, b"(\\w+)", 0);
            let mc = (api.match_context_create)(ptr::null_mut());
            (api.set_substitute_case_callout)(mc, Some(inflate), ptr::null_mut());
            let subj = b"hello world";
            let rep = b"\\U$1\\E";
            let mut cap = 0usize;
            for round in 0..5 {
                let mut out = vec![0xEEu8; cap + 200];
                let mut bl = cap;
                let rc = (api.substitute)(
                    code, subj.as_ptr(), subj.len(), 0,
                    PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                    ptr::null_mut(), mc, rep.as_ptr(), rep.len(), out.as_mut_ptr(), &mut bl,
                );
                println!("[{}] round {round} cap={cap} rc={rc} blen={}", api.name, bl as i64);
                if rc >= 0 {
                    break;
                }
                cap = bl;
            }
            (api.match_context_free)(mc);
            (api.code_free)(code);
        }
    }
}

#[test]
fn probe_dfa_many_ends() {
    let p = pair();
    unsafe {
        for api in [&p.c, &p.r] {
            let code = compile1(api, b"a|ab|abc", 0);
            for ovec in [0u32, 1, 2, 3, 4, 5] {
                for so in [0u32, PCRE2_DFA_SHORTEST, PCRE2_ENDANCHORED] {
                    let md = (api.match_data_create)(ovec, ptr::null_mut());
                    let mut ws = vec![0 as c_int; 1000];
                    let rc = (api.dfa_match)(
                        code, b"abc".as_ptr(), 3, 0, so, md, ptr::null_mut(),
                        ws.as_mut_ptr(), 1000,
                    );
                    let ov = (api.get_ovector_pointer)(md);
                    let n = if rc > 0 { 2 * rc as usize } else { 2 * ovec as usize };
                    let v: Vec<i64> = (0..n).map(|i| *ov.add(i) as i64).collect();
                    println!("[{}] ovec={ovec} so={so:#x} rc={rc} ov={v:?}", api.name);
                    (api.match_data_free)(md);
                }
            }
            (api.code_free)(code);
        }
    }
}
