mod common;
use common::*;

#[test]
fn loads_both_libraries_and_matches_version() {
    diff("version", |api, rec| unsafe {
        rec.cstring("version_str", (api.jansson_version_str)());
        for (a, b, c) in [(2, 15, 0), (2, 15, 1), (1, 0, 0), (3, 0, 0), (2, 14, 9), (2, 16, 0)] {
            rec.tag_i("cmp", (api.jansson_version_cmp)(a, b, c) as i64);
        }
        rec.tag_u("seed", *api.hashtable_seed as usize);
        rec.tag_i("dtoa_divmax", *api.dtoa_divmax as i64);
    });
}
