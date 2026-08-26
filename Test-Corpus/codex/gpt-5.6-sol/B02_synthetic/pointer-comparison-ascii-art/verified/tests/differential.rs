use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::fs::{self, File};
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::{Mutex, OnceLock};

const SHAPE_COUNT: i32 = 10;
const MAX_SHAPES: usize = 50;

#[repr(C)]
#[derive(Clone, Copy)]
struct Shape {
    type_id: c_int,
    name: [c_char; 32],
    art: [[c_char; 80]; 30],
    width: c_int,
    height: c_int,
}

#[repr(C)]
struct Scene {
    name: [c_char; 64],
    shapes: [*mut Shape; MAX_SHAPES],
    shape_count: c_int,
}

type Manager = unsafe extern "C" fn();
type ShapeGet = unsafe extern "C" fn(c_int) -> *mut Shape;
type ShapePrint = unsafe extern "C" fn(*const Shape);
type ShapeEquals = unsafe extern "C" fn(*const Shape, *const Shape) -> c_int;
type ShapeName = unsafe extern "C" fn(c_int) -> *const c_char;
type SceneCreate = unsafe extern "C" fn(*const c_char) -> *mut Scene;
type SceneDestroy = unsafe extern "C" fn(*mut Scene);
type SceneAdd = unsafe extern "C" fn(*mut Scene, *mut Shape) -> c_int;
type SceneRemove = unsafe extern "C" fn(*mut Scene, c_int) -> c_int;
type ScenePrint = unsafe extern "C" fn(*const Scene);
type SceneEquals = unsafe extern "C" fn(*const Scene, *const Scene) -> c_int;
type SceneSave = unsafe extern "C" fn(*const Scene, *const c_char) -> c_int;
type SceneLoad = unsafe extern "C" fn(*const c_char) -> *mut Scene;

struct Api {
    _library: Library,
    init: Manager,
    cleanup: Manager,
    get: ShapeGet,
    shape_print: ShapePrint,
    shape_equals: ShapeEquals,
    type_name: ShapeName,
    create: SceneCreate,
    destroy: SceneDestroy,
    add: SceneAdd,
    remove: SceneRemove,
    scene_print: ScenePrint,
    scene_equals: SceneEquals,
    save: SceneSave,
    load: SceneLoad,
    list: ScenePrint,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = Library::new(path).unwrap();
        macro_rules! symbol {
            ($name:literal, $kind:ty) => {
                *library
                    .get::<$kind>(concat!($name, "\0").as_bytes())
                    .unwrap()
            };
        }
        let api = Self {
            init: symbol!("shape_manager_init", Manager),
            cleanup: symbol!("shape_manager_cleanup", Manager),
            get: symbol!("shape_get", ShapeGet),
            shape_print: symbol!("shape_print", ShapePrint),
            shape_equals: symbol!("shape_equals", ShapeEquals),
            type_name: symbol!("shape_type_name", ShapeName),
            create: symbol!("scene_create", SceneCreate),
            destroy: symbol!("scene_destroy", SceneDestroy),
            add: symbol!("scene_add_shape", SceneAdd),
            remove: symbol!("scene_remove_shape", SceneRemove),
            scene_print: symbol!("scene_print", ScenePrint),
            scene_equals: symbol!("scene_equals", SceneEquals),
            save: symbol!("scene_save", SceneSave),
            load: symbol!("scene_load", SceneLoad),
            list: symbol!("scene_list_shapes", ScenePrint),
            _library: library,
        };
        api
    }
}

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_library() -> PathBuf {
    std::env::var_os("DRIVER_RUST_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("target/release/libdriver.so"))
}

fn c_library() -> PathBuf {
    root().join("c_src/build/libdriver_c.so")
}

fn capture<R>(fd: c_int, operation: impl FnOnce() -> R) -> (R, Vec<u8>) {
    unsafe {
        fflush(ptr::null_mut());
        let mut fds = [0; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved = dup(fd);
        assert!(saved >= 0);
        assert_eq!(dup2(fds[1], fd), fd);
        close(fds[1]);
        let result = operation();
        fflush(ptr::null_mut());
        assert_eq!(dup2(saved, fd), fd);
        close(saved);
        let mut bytes = Vec::new();
        File::from_raw_fd(fds[0]).read_to_end(&mut bytes).unwrap();
        (result, bytes)
    }
}

unsafe fn bytes(pointer: *const c_char) -> Vec<u8> {
    CStr::from_ptr(pointer).to_bytes().to_vec()
}

unsafe fn shape_snapshot(shape: *const Shape) -> (i32, Vec<u8>, i32, i32, Vec<Vec<u8>>) {
    let shape = &*shape;
    let art = (0..shape.height as usize)
        .map(|row| bytes(shape.art[row].as_ptr()))
        .collect();
    (
        shape.type_id,
        bytes(shape.name.as_ptr()),
        shape.width,
        shape.height,
        art,
    )
}

unsafe fn scene_snapshot(scene: *const Scene) -> (Vec<u8>, Vec<i32>) {
    let scene = &*scene;
    let types = (0..scene.shape_count as usize)
        .map(|index| (*scene.shapes[index]).type_id)
        .collect();
    (bytes(scene.name.as_ptr()), types)
}

unsafe fn make_scene(api: &Api, name: &[u8], types: &[i32]) -> *mut Scene {
    let name = CString::new(name).unwrap();
    let scene = (api.create)(name.as_ptr());
    assert!(!scene.is_null());
    for &type_id in types {
        assert_eq!((api.add)(scene, (api.get)(type_id)), 0);
    }
    scene
}

fn next(seed: &mut u64) -> u32 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*seed >> 32) as u32
}

fn random_name(seed: &mut u64, length: usize) -> Vec<u8> {
    (0..length)
        .map(|_| b'a' + (next(seed) % 26) as u8)
        .collect()
}

fn normalize_pointers(mut data: Vec<u8>) -> Vec<u8> {
    let marker = b"(ptr: ";
    let mut start = 0;
    while let Some(relative) = data[start..]
        .windows(marker.len())
        .position(|window| window == marker)
    {
        let value_start = start + relative + marker.len();
        if let Some(end) = data[value_start..].iter().position(|&byte| byte == b')') {
            data.splice(value_start..value_start + end, b"<address>".iter().copied());
            start = value_start + b"<address>".len();
        } else {
            break;
        }
    }
    data
}

#[test]
fn valid_path_differential_surface() {
    let _guard = lock();
    unsafe {
        let c = Api::load(&c_library());
        let rust = Api::load(&rust_library());
        (c.cleanup)();
        (rust.cleanup)();
        for type_id in 0..SHAPE_COUNT {
            assert!((c.get)(type_id).is_null());
            assert!((rust.get)(type_id).is_null());
        }
        (c.init)();
        (rust.init)();

        for type_id in 0..SHAPE_COUNT {
            let c_shape = (c.get)(type_id);
            let rust_shape = (rust.get)(type_id);
            assert_eq!(shape_snapshot(c_shape), shape_snapshot(rust_shape));
            assert_eq!(
                bytes((c.type_name)(type_id)),
                bytes((rust.type_name)(type_id))
            );
            assert_eq!(
                capture(1, || (c.shape_print)(c_shape)).1,
                capture(1, || (rust.shape_print)(rust_shape)).1
            );
            assert_eq!((c.shape_equals)(c_shape, c_shape), 1);
            assert_eq!((rust.shape_equals)(rust_shape, rust_shape), 1);
            let other = (type_id + 1) % SHAPE_COUNT;
            assert_eq!((c.shape_equals)(c_shape, (c.get)(other)), 0);
            assert_eq!((rust.shape_equals)(rust_shape, (rust.get)(other)), 0);
        }

        let null_c = (c.create)(ptr::null());
        let null_rust = (rust.create)(ptr::null());
        assert_eq!(scene_snapshot(null_c), scene_snapshot(null_rust));
        (c.destroy)(null_c);
        (rust.destroy)(null_rust);

        let mut seed = 0x5eed_cafe_d00d_beef;
        for case in 0..160 {
            let length = match case % 5 {
                0 => 0,
                1 => 1 + next(&mut seed) as usize % 62,
                2 => 63,
                3 => 64,
                _ => 65 + next(&mut seed) as usize % 40,
            };
            let name = random_name(&mut seed, length);
            let count = next(&mut seed) as usize % 51;
            let types: Vec<i32> = (0..count)
                .map(|_| (next(&mut seed) % SHAPE_COUNT as u32) as i32)
                .collect();
            let c_scene = make_scene(&c, &name, &types);
            let rust_scene = make_scene(&rust, &name, &types);
            assert_eq!(scene_snapshot(c_scene), scene_snapshot(rust_scene));

            if count > 0 {
                let index = match case % 3 {
                    0 => 0,
                    1 => count as i32 - 1,
                    _ => (next(&mut seed) as usize % count) as i32,
                };
                assert_eq!((c.remove)(c_scene, index), (rust.remove)(rust_scene, index));
                assert_eq!(scene_snapshot(c_scene), scene_snapshot(rust_scene));
            }
            (c.destroy)(c_scene);
            (rust.destroy)(rust_scene);
        }

        for index in [0, 2, 4] {
            let types = [0, 1, 2, 3, 4];
            let c_scene = make_scene(&c, b"remove", &types);
            let rust_scene = make_scene(&rust, b"remove", &types);
            assert_eq!((c.remove)(c_scene, index), (rust.remove)(rust_scene, index));
            assert_eq!(scene_snapshot(c_scene), scene_snapshot(rust_scene));
            (c.destroy)(c_scene);
            (rust.destroy)(rust_scene);
        }

        let print_cases: Vec<Vec<i32>> = std::iter::once(Vec::new())
            .chain((0..SHAPE_COUNT).map(|type_id| vec![type_id]))
            .chain(std::iter::once(vec![0, 1, 1, 9, 4, 4, 7]))
            .collect();
        for print_types in &print_cases {
            let c_print = make_scene(&c, b"Printed scene", print_types);
            let r_print = make_scene(&rust, b"Printed scene", print_types);
            assert_eq!(
                capture(1, || (c.scene_print)(c_print)).1,
                capture(1, || (rust.scene_print)(r_print)).1
            );
            let c_list = capture(1, || (c.list)(c_print)).1;
            let r_list = capture(1, || (rust.list)(r_print)).1;
            assert_eq!(normalize_pointers(c_list), normalize_pointers(r_list));
            (c.destroy)(c_print);
            (rust.destroy)(r_print);
        }

        for (first, second, expected) in [
            (&[][..], &[][..], 1),
            (&[2][..], &[2][..], 1),
            (&[1, 1, 4, 9][..], &[1, 1, 4, 9][..], 1),
            (&[1, 1, 4, 9][..], &[9, 1, 4, 1][..], 1),
            (&[1, 1, 4, 9][..], &[1, 1, 4, 8][..], 0),
            (&[1, 2][..], &[1][..], 0),
        ] {
            let c_first = make_scene(&c, b"first", first);
            let c_second = make_scene(&c, b"second", second);
            let r_first = make_scene(&rust, b"first", first);
            let r_second = make_scene(&rust, b"second", second);
            assert_eq!((c.scene_equals)(c_first, c_second), expected);
            assert_eq!((rust.scene_equals)(r_first, r_second), expected);
            assert_eq!((c.scene_equals)(c_first, c_first), 1);
            assert_eq!((rust.scene_equals)(r_first, r_first), 1);
            (c.destroy)(c_first);
            (c.destroy)(c_second);
            (rust.destroy)(r_first);
            (rust.destroy)(r_second);
        }

        for _case in 0..100 {
            let count = next(&mut seed) as usize % 51;
            let types: Vec<i32> = (0..count)
                .map(|_| (next(&mut seed) % SHAPE_COUNT as u32) as i32)
                .collect();
            let mut permuted = types.clone();
            permuted.reverse();
            let c_first = make_scene(&c, b"ignored one", &types);
            let c_second = make_scene(&c, b"ignored two", &permuted);
            let r_first = make_scene(&rust, b"ignored one", &types);
            let r_second = make_scene(&rust, b"ignored two", &permuted);
            assert_eq!(
                (c.scene_equals)(c_first, c_second),
                (rust.scene_equals)(r_first, r_second)
            );
            assert_eq!((c.scene_equals)(c_first, c_second), 1);
            if count > 0 {
                (c.remove)(c_second, 0);
                (rust.remove)(r_second, 0);
                assert_eq!(
                    (c.scene_equals)(c_first, c_second),
                    (rust.scene_equals)(r_first, r_second)
                );
            }
            for scene in [c_first, c_second] {
                (c.destroy)(scene);
            }
            for scene in [r_first, r_second] {
                (rust.destroy)(scene);
            }
        }

        let path = root().join("target/differential-scene.txt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let filename = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        for case in 0..100 {
            let length = [0, 7, 63][case % 3];
            let name = random_name(&mut seed, length);
            let count = match case {
                0 => 0,
                1 | 2 => 1,
                99 => 50,
                _ => next(&mut seed) as usize % 51,
            };
            let types: Vec<i32> = (0..count)
                .map(|_| (next(&mut seed) % SHAPE_COUNT as u32) as i32)
                .collect();
            let c_scene = make_scene(&c, &name, &types);
            let r_scene = make_scene(&rust, &name, &types);
            let c_result = capture(1, || (c.save)(c_scene, filename.as_ptr()));
            let c_file = fs::read(&path).unwrap();
            let r_result = capture(1, || (rust.save)(r_scene, filename.as_ptr()));
            let r_file = fs::read(&path).unwrap();
            assert_eq!(c_result, r_result);
            assert_eq!(c_file, r_file);

            let c_loaded = capture(1, || (c.load)(filename.as_ptr()));
            let r_loaded = capture(1, || (rust.load)(filename.as_ptr()));
            assert_eq!(c_loaded.1, r_loaded.1);
            assert_eq!(scene_snapshot(c_loaded.0), scene_snapshot(r_loaded.0));
            assert_eq!((c.scene_equals)(c_scene, c_loaded.0), 1);
            assert_eq!((rust.scene_equals)(r_scene, r_loaded.0), 1);
            (c.destroy)(c_scene);
            (rust.destroy)(r_scene);
            (c.destroy)(c_loaded.0);
            (rust.destroy)(r_loaded.0);
        }

        let fixtures: &[&[u8]] = &[
            b"negative\n-3\n",
            b"invalid types\n5\n-1\n0\n10\n9\n2147483647\n",
            b"all types\n12\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n0\n9\n",
        ];
        for fixture in fixtures {
            fs::write(&path, fixture).unwrap();
            let c_loaded = capture(1, || (c.load)(filename.as_ptr()));
            let r_loaded = capture(1, || (rust.load)(filename.as_ptr()));
            assert_eq!(c_loaded.1, r_loaded.1);
            assert_eq!(scene_snapshot(c_loaded.0), scene_snapshot(r_loaded.0));
            (c.destroy)(c_loaded.0);
            (rust.destroy)(r_loaded.0);
        }
        let mut over_capacity = b"capacity\n55\n".to_vec();
        for index in 0..55 {
            over_capacity.extend_from_slice(format!("{}\n", index % 10).as_bytes());
        }
        fs::write(&path, over_capacity).unwrap();
        let (c_loaded, c_err) = capture(2, || (c.load)(filename.as_ptr()));
        let (r_loaded, r_err) = capture(2, || (rust.load)(filename.as_ptr()));
        assert_eq!(c_err, r_err);
        assert_eq!(scene_snapshot(c_loaded), scene_snapshot(r_loaded));
        assert_eq!((*c_loaded).shape_count, 50);
        (c.destroy)(c_loaded);
        (rust.destroy)(r_loaded);

        (c.cleanup)();
        (rust.cleanup)();
        (c.cleanup)();
        (rust.cleanup)();
    }
}

#[test]
fn error_path_differential_surface() {
    let _guard = lock();
    unsafe {
        let c = Api::load(&c_library());
        let rust = Api::load(&rust_library());
        (c.init)();
        (rust.init)();

        for invalid in [-2_147_483_648, -1, 10, 11, 2_147_483_647] {
            assert!((c.get)(invalid).is_null());
            assert!((rust.get)(invalid).is_null());
            assert_eq!(
                bytes((c.type_name)(invalid)),
                bytes((rust.type_name)(invalid))
            );
        }
        assert_eq!(
            capture(1, || (c.shape_print)(ptr::null())).1,
            capture(1, || (rust.shape_print)(ptr::null())).1
        );
        assert_eq!(
            (c.shape_equals)(ptr::null(), ptr::null()),
            (rust.shape_equals)(ptr::null(), ptr::null())
        );
        assert_eq!((c.shape_equals)(ptr::null(), (c.get)(0)), 0);
        assert_eq!((rust.shape_equals)(ptr::null(), (rust.get)(0)), 0);

        assert_eq!((c.add)(ptr::null_mut(), (c.get)(0)), -1);
        assert_eq!((rust.add)(ptr::null_mut(), (rust.get)(0)), -1);
        let c_scene = make_scene(&c, b"errors", &[]);
        let r_scene = make_scene(&rust, b"errors", &[]);
        assert_eq!((c.add)(c_scene, ptr::null_mut()), -1);
        assert_eq!((rust.add)(r_scene, ptr::null_mut()), -1);
        assert_eq!((c.remove)(ptr::null_mut(), 0), -1);
        assert_eq!((rust.remove)(ptr::null_mut(), 0), -1);
        for index in [-2_147_483_648, -1, 0, 1, 2_147_483_647] {
            assert_eq!((c.remove)(c_scene, index), (rust.remove)(r_scene, index));
        }
        for index in 0..50 {
            assert_eq!((c.add)(c_scene, (c.get)(index % 10)), 0);
            assert_eq!((rust.add)(r_scene, (rust.get)(index % 10)), 0);
        }
        assert_eq!(
            capture(2, || (c.add)(c_scene, (c.get)(0))),
            capture(2, || (rust.add)(r_scene, (rust.get)(0)))
        );

        assert_eq!(
            capture(1, || (c.scene_print)(ptr::null())).1,
            capture(1, || (rust.scene_print)(ptr::null())).1
        );
        assert_eq!(
            capture(1, || (c.list)(ptr::null())).1,
            capture(1, || (rust.list)(ptr::null())).1
        );
        assert_eq!((c.scene_equals)(ptr::null(), c_scene), 0);
        assert_eq!((rust.scene_equals)(ptr::null(), r_scene), 0);
        assert_eq!((c.scene_equals)(c_scene, ptr::null()), 0);
        assert_eq!((rust.scene_equals)(r_scene, ptr::null()), 0);

        assert_eq!((c.save)(ptr::null(), ptr::null()), -1);
        assert_eq!((rust.save)(ptr::null(), ptr::null()), -1);
        let valid = CString::new("/tmp/unused-scene").unwrap();
        assert_eq!((c.save)(ptr::null(), valid.as_ptr()), -1);
        assert_eq!((rust.save)(ptr::null(), valid.as_ptr()), -1);
        assert_eq!((c.save)(c_scene, ptr::null()), -1);
        assert_eq!((rust.save)(r_scene, ptr::null()), -1);
        let bad = CString::new("/no/such/directory/scene.txt").unwrap();
        assert_eq!(
            capture(2, || (c.save)(c_scene, bad.as_ptr())),
            capture(2, || (rust.save)(r_scene, bad.as_ptr()))
        );
        assert!((c.load)(ptr::null()).is_null());
        assert!((rust.load)(ptr::null()).is_null());
        assert_eq!(
            capture(2, || (c.load)(bad.as_ptr())).1,
            capture(2, || (rust.load)(bad.as_ptr())).1
        );

        let path = root().join("target/differential-invalid.txt");
        let filename = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        for fixture in [
            b"".as_slice(),
            b"name only\n",
            b"name\nx\n",
            b"name\n2\n0\n",
        ] {
            fs::write(&path, fixture).unwrap();
            let c_loaded = (c.load)(filename.as_ptr());
            let r_loaded = (rust.load)(filename.as_ptr());
            assert_eq!(c_loaded.is_null(), r_loaded.is_null());
            assert!(c_loaded.is_null());
        }
        fs::write(&path, format!("{}\n1\n0\n", "x".repeat(70))).unwrap();
        assert!((c.load)(filename.as_ptr()).is_null());
        assert!((rust.load)(filename.as_ptr()).is_null());

        (c.destroy)(ptr::null_mut());
        (rust.destroy)(ptr::null_mut());
        (c.destroy)(c_scene);
        (rust.destroy)(r_scene);
        (c.cleanup)();
        (rust.cleanup)();
    }
}

#[test]
fn allocation_failure_differential_surface() {
    let _guard = lock();
    let build = root().join("target/fault-injection");
    fs::create_dir_all(&build).unwrap();
    let preload = build.join("libmalloc_fail.so");
    let runner = build.join("fault_runner");
    assert!(Command::new("cc")
        .args(["-fPIC", "-shared", "tests/malloc_fail.c", "-o"])
        .arg(&preload)
        .current_dir(root())
        .status()
        .unwrap()
        .success());
    assert!(Command::new("cc")
        .args(["-I", "c_src/include", "tests/fault_runner.c", "-ldl", "-o"])
        .arg(&runner)
        .current_dir(root())
        .status()
        .unwrap()
        .success());

    let fixture = build.join("scene.txt");
    fs::write(&fixture, b"name\n0\n").unwrap();
    for mode in ["shape-init", "scene-create", "scene-load"] {
        let run = |library: &Path| {
            let mut command = Command::new(&runner);
            command.env("LD_PRELOAD", &preload).arg(library).arg(mode);
            if mode == "scene-load" {
                command.arg(&fixture);
            }
            command.output().unwrap()
        };
        let c = run(&c_library());
        let rust = run(&rust_library());
        assert_eq!(c.status.code(), rust.status.code(), "{mode}");
        assert_eq!(c.stdout, rust.stdout, "{mode}");
        assert_eq!(c.stderr, rust.stderr, "{mode}");
        if mode == "shape-init" {
            assert_eq!(c.status.code(), Some(1));
            assert_eq!(c.stderr, b"Error: Failed to allocate shape\n");
        } else {
            assert!(c.status.success());
        }
    }
}
