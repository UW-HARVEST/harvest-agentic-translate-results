use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCES: &[&str] = &[
    "dtoa.c",
    "dump.c",
    "error.c",
    "hashtable.c",
    "hashtable_seed.c",
    "load.c",
    "memory.c",
    "pack_unpack.c",
    "strbuffer.c",
    "strconv.c",
    "utf.c",
    "value.c",
    "version.c",
];

const FUNCTIONS: &[&str] = &[
    "do_deep_copy",
    "do_object_update_recursive",
    "dtoa",
    "dtoa_r",
    "freedtoa",
    "gethex",
    "hashtable_clear",
    "hashtable_close",
    "hashtable_del",
    "hashtable_get",
    "hashtable_init",
    "hashtable_iter",
    "hashtable_iter_at",
    "hashtable_iter_key",
    "hashtable_iter_key_len",
    "hashtable_iter_next",
    "hashtable_iter_set",
    "hashtable_iter_value",
    "hashtable_set",
    "jansson_version_cmp",
    "jansson_version_str",
    "json_array",
    "json_array_append_new",
    "json_array_clear",
    "json_array_extend",
    "json_array_get",
    "json_array_insert_new",
    "json_array_remove",
    "json_array_set_new",
    "json_array_size",
    "json_copy",
    "json_deep_copy",
    "json_delete",
    "json_dump_callback",
    "json_dump_file",
    "json_dumpb",
    "json_dumpf",
    "json_dumpfd",
    "json_dumps",
    "json_equal",
    "json_false",
    "json_get_alloc_funcs",
    "json_get_alloc_funcs2",
    "json_integer",
    "json_integer_set",
    "json_integer_value",
    "json_load_callback",
    "json_load_file",
    "json_loadb",
    "json_loadf",
    "json_loadfd",
    "json_loads",
    "json_null",
    "json_number_value",
    "json_object",
    "json_object_clear",
    "json_object_del",
    "json_object_deln",
    "json_object_get",
    "json_object_getn",
    "json_object_iter",
    "json_object_iter_at",
    "json_object_iter_key",
    "json_object_iter_key_len",
    "json_object_iter_next",
    "json_object_iter_set_new",
    "json_object_iter_value",
    "json_object_key_to_iter",
    "json_object_seed",
    "json_object_set_new",
    "json_object_set_new_nocheck",
    "json_object_setn_new",
    "json_object_setn_new_nocheck",
    "json_object_size",
    "json_object_update",
    "json_object_update_existing",
    "json_object_update_missing",
    "json_object_update_recursive",
    "json_pack",
    "json_pack_ex",
    "json_real",
    "json_real_set",
    "json_real_value",
    "json_set_alloc_funcs",
    "json_set_alloc_funcs2",
    "json_sprintf",
    "json_string",
    "json_string_length",
    "json_string_nocheck",
    "json_string_set",
    "json_string_set_nocheck",
    "json_string_setn",
    "json_string_setn_nocheck",
    "json_string_value",
    "json_stringn",
    "json_stringn_nocheck",
    "json_true",
    "json_unpack",
    "json_unpack_ex",
    "json_vpack_ex",
    "json_vsprintf",
    "json_vunpack_ex",
    "jsonp_dtostr",
    "jsonp_error_init",
    "jsonp_error_set",
    "jsonp_error_set_source",
    "jsonp_error_vset",
    "jsonp_free",
    "jsonp_loop_check",
    "jsonp_malloc",
    "jsonp_realloc",
    "jsonp_stringn_nocheck_own",
    "jsonp_strndup",
    "jsonp_strtod",
    "strbuffer_append_byte",
    "strbuffer_append_bytes",
    "strbuffer_clear",
    "strbuffer_close",
    "strbuffer_init",
    "strbuffer_pop",
    "strbuffer_steal_value",
    "strbuffer_value",
    "strtod__unused",
    "utf8_check_first",
    "utf8_check_full",
    "utf8_check_string",
    "utf8_encode",
    "utf8_iterate",
];

fn run(command: &mut Command) {
    let status = command.status().expect("failed to execute build command");
    assert!(status.success(), "build command failed with {status}");
}

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let source_root = manifest.join("../c_src");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let rename_header = out.join("rename.h");
    let mut renames = String::from("#ifndef RUST_JANSSON_RENAME_H\n#define RUST_JANSSON_RENAME_H\n");
    for function in FUNCTIONS {
        renames.push_str(&format!("#define {function} rust_impl_{function}\n"));
    }
    renames.push_str("#endif\n");
    fs::write(&rename_header, renames).unwrap();

    let mut objects = Vec::new();
    for source in SOURCES {
        let object = out.join(format!("{source}.o"));
        let original_source_path = source_root.join("src").join(source);
        let source_path = match *source {
            "dtoa.c" => {
                let generated = out.join(source);
                let input = fs::read_to_string(&original_source_path).unwrap();
                let output = input.replace(
                    "int dtoa_divmax = 2;",
                    "extern int dtoa_divmax;",
                );
                assert_ne!(input, output);
                fs::write(&generated, output).unwrap();
                generated
            }
            "hashtable_seed.c" => {
                let generated = out.join(source);
                let input = fs::read_to_string(&original_source_path).unwrap();
                let output = input.replace(
                    "volatile uint32_t hashtable_seed = 0;",
                    "extern volatile uint32_t hashtable_seed;",
                );
                assert_ne!(input, output);
                fs::write(&generated, output).unwrap();
                generated
            }
            _ => original_source_path.clone(),
        };
        let mut cc = Command::new("cc");
        cc.arg("-std=c99")
            .arg("-O3")
            .arg("-fPIC")
            .arg("-w")
            .arg("-DHAVE_CONFIG_H")
            .arg("-include")
            .arg(&rename_header)
            .arg("-I")
            .arg(source_root.join("include"))
            .arg("-I")
            .arg(source_root.join("src"))
            .arg("-c")
            .arg(&source_path)
            .arg("-o")
            .arg(&object);
        run(&mut cc);
        objects.push(object);
        println!("cargo:rerun-if-changed={}", original_source_path.display());
    }

    let archive = out.join("libjansson_impl.a");
    let mut ar = Command::new("ar");
    ar.arg("crus").arg(&archive);
    for object in &objects {
        ar.arg(object);
    }
    run(&mut ar);

    println!("cargo:rerun-if-changed={}", source_root.join("include").display());
    println!("cargo:rerun-if-changed={}", source_root.join("src").display());
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=jansson_impl");

    assert!(Path::new(&archive).is_file());
}
