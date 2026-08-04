use recordManager::dberror::RC;
use recordManager::rm_serializer::{
    attr_offset, serialize_attr, serialize_record, serialize_schema, serialize_value,
    string_to_value,
};
use recordManager::tables::{DataType, Record, Schema, Value, ValueUnion, RID};

fn make_test_schema() -> Schema {
    Schema {
        num_attr: 3,
        attr_names: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        data_types: vec![DataType::DtInt, DataType::DtString, DataType::DtInt],
        type_length: vec![0, 4, 0],
        key_attrs: vec![0],
        key_size: 1,
    }
}

#[test]
fn test_attr_offset() {
    let schema = make_test_schema();
    let mut o = -1i32;
    let rc = attr_offset(&schema, 0, &mut o);
    assert_eq!(rc, RC::Ok);
    assert_eq!(o, 0);

    let mut o2 = -1i32;
    let rc = attr_offset(&schema, 1, &mut o2);
    assert_eq!(rc, RC::Ok);
    assert_eq!(o2, 4); // sizeof(int)

    let mut o3 = -1i32;
    let rc = attr_offset(&schema, 2, &mut o3);
    assert_eq!(rc, RC::Ok);
    assert_eq!(o3, 8); // sizeof(int)+ string len 4
}

#[test]
fn test_serialize_value_int() {
    let v = Value {
        dt: DataType::DtInt,
        v: ValueUnion::IntV(10),
    };
    assert_eq!(serialize_value(&v), "10");
}

#[test]
fn test_serialize_value_float() {
    let v = Value {
        dt: DataType::DtFloat,
        v: ValueUnion::FloatV(5.3),
    };
    assert_eq!(serialize_value(&v), "5.300000");
}

#[test]
fn test_serialize_value_string() {
    let v = Value {
        dt: DataType::DtString,
        v: ValueUnion::StringV("Hello World".to_string()),
    };
    assert_eq!(serialize_value(&v), "Hello World");
}

#[test]
fn test_serialize_value_bool() {
    let vt = Value {
        dt: DataType::DtBool,
        v: ValueUnion::BoolV(true),
    };
    assert_eq!(serialize_value(&vt), "true");
    let vf = Value {
        dt: DataType::DtBool,
        v: ValueUnion::BoolV(false),
    };
    assert_eq!(serialize_value(&vf), "false");
}

#[test]
fn test_string_to_value_int() {
    let v = string_to_value("i10");
    assert!(matches!(v.dt, DataType::DtInt));
    if let ValueUnion::IntV(i) = v.v {
        assert_eq!(i, 10);
    }
}

#[test]
fn test_string_to_value_float() {
    let v = string_to_value("f5.3");
    assert!(matches!(v.dt, DataType::DtFloat));
    if let ValueUnion::FloatV(f) = v.v {
        assert!((f - 5.3).abs() < 1e-3);
    }
}

#[test]
fn test_string_to_value_string() {
    let v = string_to_value("sHello World");
    assert!(matches!(v.dt, DataType::DtString));
    if let ValueUnion::StringV(s) = v.v {
        assert_eq!(s, "Hello World");
    }
}

#[test]
fn test_string_to_value_bool() {
    let vt = string_to_value("bt");
    if let ValueUnion::BoolV(b) = vt.v {
        assert!(b);
    }
    let vt2 = string_to_value("btrue");
    if let ValueUnion::BoolV(b) = vt2.v {
        assert!(b);
    }
    let vf = string_to_value("bf");
    if let ValueUnion::BoolV(b) = vf.v {
        assert!(!b);
    }
}

#[test]
fn test_string_to_value_round_trip() {
    let v = string_to_value("i10");
    assert_eq!(serialize_value(&v), "10");

    let v = string_to_value("f5.3");
    assert_eq!(serialize_value(&v), "5.300000");

    let v = string_to_value("sHello World");
    assert_eq!(serialize_value(&v), "Hello World");

    let v = string_to_value("bt");
    assert_eq!(serialize_value(&v), "true");
}

#[test]
fn test_serialize_schema() {
    let s = make_test_schema();
    let out = serialize_schema(&s);
    assert_eq!(
        out,
        "Schema with <3> attributes (a: INT, b: STRING[4], c: INT) with keys: (a)\n"
    );
}

#[test]
fn test_serialize_record_and_attr() {
    let schema = make_test_schema();
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&1i32.to_ne_bytes());
    data.extend_from_slice(b"aaaa");
    data.extend_from_slice(&3i32.to_ne_bytes());
    let r = Record {
        id: RID { page: 0, slot: 0 },
        data: unsafe { String::from_utf8_unchecked(data) },
    };
    assert_eq!(serialize_record(&r, &schema), "[0-0] (a:1b:aaaa,c:3,)");
    assert_eq!(serialize_attr(&r, &schema, 0), "a:1");
    assert_eq!(serialize_attr(&r, &schema, 1), "b:aaaa");
    assert_eq!(serialize_attr(&r, &schema, 2), "c:3");
}

#[test]
fn test_serialize_attr_float() {
    // Build a one-attr float schema
    let schema = Schema {
        num_attr: 1,
        attr_names: vec!["x".to_string()],
        data_types: vec![DataType::DtFloat],
        type_length: vec![0],
        key_attrs: vec![],
        key_size: 0,
    };
    let mut data = Vec::new();
    data.extend_from_slice(&3.14f32.to_ne_bytes());
    let r = Record {
        id: RID { page: 0, slot: 0 },
        data: unsafe { String::from_utf8_unchecked(data) },
    };
    assert_eq!(serialize_attr(&r, &schema, 0), "x:3.140000");
}

#[test]
fn test_serialize_attr_bool() {
    let schema = Schema {
        num_attr: 1,
        attr_names: vec!["b".to_string()],
        data_types: vec![DataType::DtBool],
        type_length: vec![0],
        key_attrs: vec![],
        key_size: 0,
    };
    let r = Record {
        id: RID { page: 1, slot: 2 },
        data: unsafe { String::from_utf8_unchecked(vec![1u8]) },
    };
    assert_eq!(serialize_attr(&r, &schema, 0), "b:TRUE");

    let r2 = Record {
        id: RID { page: 1, slot: 2 },
        data: unsafe { String::from_utf8_unchecked(vec![0u8]) },
    };
    assert_eq!(serialize_attr(&r2, &schema, 0), "b:FALSE");
}

fn main() {}
