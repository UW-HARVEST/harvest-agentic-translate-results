//! Phase A sanity: both shared objects load and every symbol resolves.
mod harness;
use harness::*;

#[test]
fn both_libraries_load_and_all_symbols_resolve() {
    let p = pair();
    unsafe {
        let cv = read_cstr((p.c.cJSON_Version)()).unwrap();
        let rv = read_cstr((p.r.cJSON_Version)()).unwrap();
        assert_eq!(cv, rv, "cJSON_Version differs");
        assert_eq!(cv, b"1.7.19".to_vec());
    }
}

#[test]
fn smoke_parse_print_matches() {
    let p = pair();
    let src = cstr("{\"a\":[1,2,{\"b\":null}],\"c\":\"x\\ty\"}");
    unsafe {
        let ci = (p.c.cJSON_Parse)(sp(&src));
        let ri = (p.r.cJSON_Parse)(sp(&src));
        assert!(!ci.is_null() && !ri.is_null());
        assert_snap_eq(ci, ri, "smoke");
        let co = take_print(&p.c, (p.c.cJSON_Print)(ci));
        let ro = take_print(&p.r, (p.r.cJSON_Print)(ri));
        assert_eq!(co, ro);
        (p.c.cJSON_Delete)(ci);
        (p.r.cJSON_Delete)(ri);
    }
}
