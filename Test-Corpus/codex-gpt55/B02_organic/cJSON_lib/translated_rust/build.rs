use std::env;
use std::path::PathBuf;
use std::process::Command;

const EXPORTED: &[&str] = &[
    "cJSON_GetErrorPtr",
    "cJSON_GetStringValue",
    "cJSON_GetNumberValue",
    "cJSON_Version",
    "cJSON_InitHooks",
    "cJSON_Delete",
    "cJSON_SetNumberHelper",
    "cJSON_SetValuestring",
    "cJSON_ParseWithOpts",
    "cJSON_ParseWithLengthOpts",
    "cJSON_Parse",
    "cJSON_ParseWithLength",
    "cJSON_Print",
    "cJSON_PrintUnformatted",
    "cJSON_PrintBuffered",
    "cJSON_PrintPreallocated",
    "cJSON_GetArraySize",
    "cJSON_GetArrayItem",
    "cJSON_GetObjectItem",
    "cJSON_GetObjectItemCaseSensitive",
    "cJSON_HasObjectItem",
    "cJSON_AddItemToArray",
    "cJSON_AddItemToObject",
    "cJSON_AddItemToObjectCS",
    "cJSON_AddItemReferenceToArray",
    "cJSON_AddItemReferenceToObject",
    "cJSON_AddNullToObject",
    "cJSON_AddTrueToObject",
    "cJSON_AddFalseToObject",
    "cJSON_AddBoolToObject",
    "cJSON_AddNumberToObject",
    "cJSON_AddStringToObject",
    "cJSON_AddRawToObject",
    "cJSON_AddObjectToObject",
    "cJSON_AddArrayToObject",
    "cJSON_DetachItemViaPointer",
    "cJSON_DetachItemFromArray",
    "cJSON_DeleteItemFromArray",
    "cJSON_DetachItemFromObject",
    "cJSON_DetachItemFromObjectCaseSensitive",
    "cJSON_DeleteItemFromObject",
    "cJSON_DeleteItemFromObjectCaseSensitive",
    "cJSON_InsertItemInArray",
    "cJSON_ReplaceItemViaPointer",
    "cJSON_ReplaceItemInArray",
    "cJSON_ReplaceItemInObject",
    "cJSON_ReplaceItemInObjectCaseSensitive",
    "cJSON_CreateNull",
    "cJSON_CreateTrue",
    "cJSON_CreateFalse",
    "cJSON_CreateBool",
    "cJSON_CreateNumber",
    "cJSON_CreateString",
    "cJSON_CreateStringReference",
    "cJSON_CreateObjectReference",
    "cJSON_CreateArrayReference",
    "cJSON_CreateRaw",
    "cJSON_CreateArray",
    "cJSON_CreateObject",
    "cJSON_CreateIntArray",
    "cJSON_CreateFloatArray",
    "cJSON_CreateDoubleArray",
    "cJSON_CreateStringArray",
    "cJSON_Duplicate",
    "cJSON_Minify",
    "cJSON_IsInvalid",
    "cJSON_IsFalse",
    "cJSON_IsTrue",
    "cJSON_IsBool",
    "cJSON_IsNull",
    "cJSON_IsNumber",
    "cJSON_IsString",
    "cJSON_IsArray",
    "cJSON_IsObject",
    "cJSON_IsRaw",
    "cJSON_Compare",
    "cJSON_malloc",
    "cJSON_free",
    "driver",
];

fn run(mut command: Command) {
    let status = command.status().expect("failed to run command");
    assert!(status.success(), "command failed with status {status}: {command:?}");
}

fn main() {
    println!("cargo:rerun-if-changed=c_src/cJSON.c");
    println!("cargo:rerun-if-changed=c_src/cJSON.h");
    println!("cargo:rerun-if-changed=c_src/test.c");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let cc = env::var("CC").unwrap_or_else(|_| "cc".to_owned());
    let ar = env::var("AR").unwrap_or_else(|_| "ar".to_owned());
    let cjson_o = out_dir.join("cJSON.o");
    let test_o = out_dir.join("test.o");
    let lib = out_dir.join("libcjson_internal.a");

    for (source, object) in [("c_src/cJSON.c", &cjson_o), ("c_src/test.c", &test_o)] {
        let mut command = Command::new(&cc);
        command
            .arg("-std=c89")
            .arg("-fPIC")
            .arg("-fvisibility=hidden")
            .arg("-I")
            .arg("c_src")
            .arg("-c")
            .arg(source)
            .arg("-o")
            .arg(object);

        for symbol in EXPORTED {
            command.arg(format!("-D{symbol}=rust_internal_{symbol}"));
        }

        run(command);
    }

    let mut command = Command::new(&ar);
    command.arg("crus").arg(&lib).arg(&cjson_o).arg(&test_o);
    run(command);

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=cjson_internal");
    println!("cargo:rustc-link-lib=m");
}

