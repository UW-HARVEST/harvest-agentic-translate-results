//! Reconnaissance: which of the ten C `assert()` sites and six soft-error sites
//! are actually reachable through `pinflate`, and with what inputs. Prints a
//! histogram; used to derive the targeted Phase C rows. Also a differential test
//! in its own right (row C42 of CONFIGS.md).

mod common;

use common::{Case, Diff};
use std::collections::BTreeMap;

#[test]
fn recon() {
    let mut d = Diff::new();
    let mut rng = common::Rng::new(0xFEED_F00D);
    let mut sites: BTreeMap<String, (usize, Vec<u8>, usize, i32)> = BTreeMap::new();
    let mut soft: BTreeMap<String, (usize, Vec<u8>)> = BTreeMap::new();
    let mut oks = 0usize;
    let mut sigsegv = 0usize;

    let b = d.row_start("recon / random malformed inputs");
    for i in 0..6000u32 {
        let n = match i % 6 {
            0 => 1,
            1 => 2,
            2 => 3,
            3 => 4,
            4 => rng.range(5, 12) as usize,
            _ => rng.range(1, 40) as usize,
        };
        let input = rng.bytes(n);
        let ia = rng.below(4) as usize;
        let ob = [0i32, 1, 4, 64, 1024][rng.below(5) as usize];
        let case = Case::new(input.clone(), ob).in_align(ia);
        let c = d.check("recon", "random bytes", &case);
        if let Some(a) = &c.assert_site {
            let e = sites
                .entry(a.clone())
                .or_insert((0, input.clone(), ia, ob));
            e.0 += 1;
        } else if let Some(s) = c.signal {
            if s == libc::SIGSEGV {
                sigsegv += 1;
            }
        } else if c.ret == 0 {
            let msg = String::from_utf8_lossy(c.err.as_deref().unwrap_or(b"<null>")).into_owned();
            let e = soft.entry(msg).or_insert((0, input.clone()));
            e.0 += 1;
        } else {
            oks += 1;
        }
    }
    d.row_end(b);

    println!("\n--- assert sites reached ({} distinct) ---", sites.len());
    for (k, v) in &sites {
        println!("  {:5}x  {k}\n           e.g. input={} in_align={} out_bytes={}", v.0, common::hex(&v.1), v.2, v.3);
    }
    println!("\n--- soft errors reached ({} distinct) ---", soft.len());
    for (k, v) in &soft {
        println!("  {:5}x  {k}\n           e.g. input={}", v.0, common::hex(&v.1));
    }
    println!("\nret==1: {oks}, SIGSEGV: {sigsegv}");

    d.finish("recon");
}
