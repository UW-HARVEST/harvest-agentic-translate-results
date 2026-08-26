// GENERATED from dev_tests/compare.py - the curated differential scenarios
// for the `driver` executable.  Do not edit by hand; see tests/binary_diff.rs.
#[allow(clippy::type_complexity)]
fn curated_cases() -> Vec<(String, Vec<u8>, Vec<(&'static str, &'static [u8])>)> {
    let mut v: Vec<(String, Vec<u8>, Vec<(&'static str, &'static [u8])>)> = Vec::new();
    v.push(("add_invalid".to_string(), b"3\n2\nA\n3\n5\n0\n3\n0\n99\n3\n0\n-1\n3\n0\nzz\n12\n".to_vec(), vec![]));
    v.push(("add_shapes".to_string(), b"2\nA\n3\n0\n0\n3\n0\n9\n5\n0\n12\n".to_vec(), vec![]));
    v.push(("compare_scenes".to_string(), b"2\nA\n2\nB\n3\n0\n1\n3\n1\n1\n10\n0\n1\n3\n0\n2\n10\n0\n1\n12\n".to_vec(), vec![]));
    v.push(("compare_scenes_bad".to_string(), b"2\nA\n2\nB\n10\n5\n0\n10\n0\n-1\n12\n".to_vec(), vec![]));
    v.push(("compare_scenes_few".to_string(), b"10\n2\nA\n10\n12\n".to_vec(), vec![]));
    v.push(("compare_shapes".to_string(), b"9\n0\n0\n9\n1\n2\n9\n99\n1\n9\n0\n-1\n12\n".to_vec(), vec![]));
    v.push(("compare_shapes_bad".to_string(), b"9\nabc\n9\n0\nxyz\n12\n".to_vec(), vec![]));
    v.push(("create_empty_name".to_string(), b"2\n\n6\n12\n".to_vec(), vec![]));
    v.push(("create_scenes".to_string(), b"2\nMy Scene\n2\nOther\n6\n12\n".to_vec(), vec![]));
    v.push(("delete_none".to_string(), b"11\n12\n".to_vec(), vec![]));
    v.push(("delete_scene".to_string(), b"2\nA\n2\nB\n2\nC\n11\n1\n6\n11\n9\n11\n-1\n11\n0\n6\n12\n".to_vec(), vec![]));
    v.push(("empty".to_string(), b"".to_vec(), vec![]));
    v.push(("exit_only".to_string(), b"12\n".to_vec(), vec![]));
    v.push(("huge_numbers".to_string(), b"2147483648\n99999999999999999999\n-99999999999999999999\n4294967308\n12\n".to_vec(), vec![]));
    v.push(("invalid_choice".to_string(), b"0\n13\n99\n-5\n12\n".to_vec(), vec![]));
    v.push(("invalid_input".to_string(), b"abc\n\n   \nx1\n12\n".to_vec(), vec![]));
    v.push(("leading_space".to_string(), b"   6\n  +12\n".to_vec(), vec![]));
    v.push(("list_empty".to_string(), b"6\n12\n".to_vec(), vec![]));
    v.push(("load_51".to_string(), b"8\nscene.dat\n5\n0\n12\n".to_vec(), vec![("scene.dat", &b"Big\n55\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n"[..])]));
    v.push(("load_bad_count".to_string(), b"8\nscene.dat\n6\n12\n".to_vec(), vec![("scene.dat", &b"Name\nxyz\n"[..])]));
    v.push(("load_bad_types".to_string(), b"8\nscene.dat\n5\n0\n12\n".to_vec(), vec![("scene.dat", &b"S\n4\n0\n99\n-3\n7\n"[..])]));
    v.push(("load_crlf".to_string(), b"8\nscene.dat\n5\n0\n12\n".to_vec(), vec![("scene.dat", &b"S\r\n1\r\n3\r\n"[..])]));
    v.push(("load_empty_file".to_string(), b"8\nscene.dat\n6\n12\n".to_vec(), vec![("scene.dat", &b""[..])]));
    v.push(("load_long_name".to_string(), b"8\nscene.dat\n5\n0\n12\n".to_vec(), vec![("scene.dat", &b"NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN\n2\n1\n2\n"[..])]));
    v.push(("load_missing".to_string(), b"8\nnope.txt\n12\n".to_vec(), vec![]));
    v.push(("load_no_trailing_nl".to_string(), b"8\nscene.dat\n5\n0\n12\n".to_vec(), vec![("scene.dat", &b"S\n2\n1\n2"[..])]));
    v.push(("load_ok".to_string(), b"8\nscene.dat\n5\n0\n6\n12\n".to_vec(), vec![("scene.dat", &b"Loaded Scene\n3\n0\n5\n9\n"[..])]));
    v.push(("load_only_name".to_string(), b"8\nscene.dat\n6\n12\n".to_vec(), vec![("scene.dat", &b"Only Name\n"[..])]));
    v.push(("load_short".to_string(), b"8\nscene.dat\n5\n0\n12\n".to_vec(), vec![("scene.dat", &b"S\n5\n1\n2\n"[..])]));
    v.push(("load_spaces".to_string(), b"8\nscene.dat\n5\n0\n12\n".to_vec(), vec![("scene.dat", &b"S\n  2  \n   1    2   \n"[..])]));
    v.push(("long_menu_line".to_string(), b"1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\n12\n".to_vec(), vec![]));
    v.push(("long_name".to_string(), b"2\nNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN\n6\n12\n".to_vec(), vec![]));
    v.push(("max_scenes".to_string(), b"2\nA\n2\nA\n2\nA\n2\nA\n2\nA\n2\nA\n2\nA\n2\nA\n2\nA\n2\nA\n2\nA\n2\nA\n6\n12\n".to_vec(), vec![]));
    v.push(("max_scenes_load".to_string(), b"2\nA\n2\nA\n2\nA\n2\nA\n2\nA\n2\nA\n2\nA\n2\nA\n2\nA\n2\nA\n8\nscene.dat\n12\n".to_vec(), vec![("scene.dat", &b"S\n1\n0\n"[..])]));
    v.push(("mixed".to_string(), b"1\n2\nFarm\n3\n0\n1\n3\n0\n0\n3\n0\n2\n5\n0\n6\n7\n0\nfarm.sav\n8\nfarm.sav\n10\n0\n1\n4\n0\n1\n5\n0\n11\n0\n6\n12\n".to_vec(), vec![]));
    v.push(("no_newline_end".to_string(), b"1".to_vec(), vec![]));
    v.push(("remove_empty".to_string(), b"2\nA\n4\n0\n12\n".to_vec(), vec![]));
    v.push(("remove_no_scene".to_string(), b"4\n12\n".to_vec(), vec![]));
    v.push(("remove_shapes".to_string(), b"2\nA\n3\n0\n7\n3\n0\n2\n4\n0\n1\n4\n0\n5\n4\n0\n0\n5\n0\n12\n".to_vec(), vec![]));
    v.push(("save_bad_file".to_string(), b"2\nA\n7\n0\n/nonexistent_dir/x/y.txt\n12\n".to_vec(), vec![]));
    v.push(("save_empty_name".to_string(), b"2\nA\n7\n0\n\n12\n".to_vec(), vec![]));
    v.push(("save_load".to_string(), b"2\nScene1\n3\n0\n0\n3\n0\n7\n7\n0\nout.txt\n8\nout.txt\n5\n1\n6\n12\n".to_vec(), vec![]));
    v.push(("save_no_scene".to_string(), b"7\n12\n".to_vec(), vec![]));
    v.push(("scanf_across_lines".to_string(), b"3\n2\nA\n3\n\n\n0\n\n\n1\n6\n12\n".to_vec(), vec![]));
    v.push(("scanf_multi_on_line".to_string(), b"2\nA\n3\n0 5\n5\n0\n12\n".to_vec(), vec![]));
    v.push(("shape_50_full".to_string(), b"2\nA\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n3\n0\n0\n5\n0\n12\n".to_vec(), vec![]));
    v.push(("tab_input".to_string(), b"\t6\n\t12\n".to_vec(), vec![]));
    v.push(("trailing_junk".to_string(), b"12abc\n".to_vec(), vec![]));
    v.push(("view_bad_idx".to_string(), b"2\nA\n5\n7\n5\n-1\n5\n0\n12\n".to_vec(), vec![]));
    v.push(("view_no_scene".to_string(), b"5\n12\n".to_vec(), vec![]));
    v.push(("view_shapes".to_string(), b"1\n12\n".to_vec(), vec![]));
    v
}
