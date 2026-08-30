mod common;
use common::*;

#[test]
fn smoke_config_and_load() {
    let p = pair();
    p.check_config();
    println!("config={} N={} SPX_BYTES={} CTX_BYTES={}", cfg_name(), N, SPX_BYTES, CTX_BYTES);
}
