mod common;
use common::*;
use std::ptr;

#[test]
fn maketables_output_identical() {
    let p = pair();
    unsafe {
        let a = (p.c.maketables)(ptr::null_mut());
        let b = (p.r.maketables)(ptr::null_mut());
        assert!(!a.is_null() && !b.is_null());
        let mut n = 0u32;
        (p.c.config)(PCRE2_CONFIG_TABLES_LENGTH, &mut n as *mut u32 as Ptr);
        let n = n as usize;
        println!("tables length = {n}");
        let sa = std::slice::from_raw_parts(a, n);
        let sb = std::slice::from_raw_parts(b, n);
        // also compare against the built-in default tables
        let da = std::slice::from_raw_parts(p.c.data("_pcre2_default_tables_8"), n);

        let diffs: Vec<usize> = (0..n).filter(|&i| sa[i] != sb[i]).collect();
        if !diffs.is_empty() {
            // PCRE2 tables layout: lcc[256] fcc[256] cbits[cbit_length] ctypes[256]
            println!("differing offsets ({} total): {:?}", diffs.len(), &diffs[..diffs.len().min(40)]);
            for &i in diffs.iter().take(20) {
                let region = if i < 256 {
                    format!("lcc[{i}]")
                } else if i < 512 {
                    format!("fcc[{}]", i - 256)
                } else if i < n - 256 {
                    format!("cbits[{}] (byte {} of set {})", i - 512, (i - 512) % 32, (i - 512) / 32)
                } else {
                    format!("ctypes[{}]", i - (n - 256))
                };
                println!(
                    "  off {i:5} {region:32} C={:#04x} rust={:#04x} default={:#04x}",
                    sa[i], sb[i], da[i]
                );
            }
        }
        println!("C == default tables: {}", sa == da);
        println!("rust == default tables: {}", sb == da);
        (p.c.maketables_free)(ptr::null_mut(), a);
        (p.r.maketables_free)(ptr::null_mut(), b);
        assert!(diffs.is_empty(), "pcre2_maketables_8 output differs in {} bytes", diffs.len());
    }
}
