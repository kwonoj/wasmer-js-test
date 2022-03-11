use std::sync::Arc;

use parking_lot::Mutex;
use rkyv::{AlignedVec, archived_root, Deserialize};
use runner_base::SharedStruct;
use wasmer::{
    imports, Array, Function, Instance, LazyInit, Memory, Module, Store, WasmPtr, WasmerEnv,
};

static WASM_BINARY: &'static [u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/debug/wasm_guest_binary.wasm");

#[derive(WasmerEnv, Clone)]
struct HostEnvironment {
    #[wasmer(export)]
    memory: wasmer::LazyInit<Memory>,
    transform_result: Arc<Mutex<Vec<u8>>>,
}

fn set_transform_result(env: &HostEnvironment, bytes_ptr: i32, bytes_ptr_len: i32) {
    let ptr: WasmPtr<u8, Array> = WasmPtr::new(bytes_ptr as _);

    let memory = env.memory_ref().unwrap();
    let derefed_ptr = ptr
        .deref(memory, 0, bytes_ptr_len as u32)
        .expect("Should able to deref from given ptr");

    let ret = derefed_ptr
        .iter()
        .enumerate()
        .take(bytes_ptr_len as usize)
        .map(|(_size, cell)| cell.get())
        .collect::<Vec<u8>>();

    (*env.transform_result.lock()) = ret;
}

fn write_bytes_into_guest(instance: &Instance, bytes: &AlignedVec) -> (i32, i32) {
    let memory = instance.exports.get_memory("memory").unwrap();

    let alloc = instance
        .exports
        .get_native_function::<u32, i32>("__alloc")
        .unwrap();
    let bytes_len = bytes.len();

    let ptr_start = alloc.call(bytes_len.try_into().unwrap()).unwrap();

    let view = memory.view();

    unsafe {
        view.subarray(
            ptr_start.try_into().unwrap(),
            ptr_start as u32 + bytes_len as u32,
        )
        .copy_from(bytes);
    }

    (ptr_start, bytes_len.try_into().unwrap())
}

fn read_bytes_from_guest(result: &Arc<Mutex<Vec<u8>>>) -> SharedStruct {
    let bytes = &(*result.lock());
    let mut aligned_vec = AlignedVec::with_capacity(bytes.len());
    aligned_vec.extend_from_slice(bytes);

    let deserialized = unsafe { archived_root::<SharedStruct>(&aligned_vec[..]) };
    deserialized.deserialize(&mut rkyv::Infallible).unwrap()
}

fn load_plugin() -> Result<(Instance, Arc<Mutex<Vec<u8>>>), std::fmt::Error> {
    let wasmer_store = Store::default();
    let module = Module::new(&wasmer_store, &WASM_BINARY);

    return match module {
        Ok(module) => {
            let transform_result: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(vec![]));

            let set_transform_result_fn_decl = Function::new_native_with_env(
                &wasmer_store,
                HostEnvironment {
                    memory: LazyInit::default(),
                    transform_result: transform_result.clone(),
                },
                set_transform_result,
            );

            let imports = imports! {
                "env" => {
                    "__set_transform_result" => set_transform_result_fn_decl,
                }
            };

            let instance = Instance::new(&module, &imports).unwrap();

            Ok((instance, transform_result))
        },
        Err(err) => panic!("should not be here"),
    }
}

pub fn test_success() {
    let input = SharedStruct {
        name: "input".to_string(),
        list: vec!["input1".to_string(), "input2".to_string()],
        other_list: vec![1, 2, 3, 4],
    };

    let input_serialized = rkyv::to_bytes::<_, 512>(&input).unwrap();

    let wasmer_store = Store::default();
    let module = Module::new(&wasmer_store, &WASM_BINARY).unwrap();
    let transform_result: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(vec![]));

    let set_transform_result_fn_decl = Function::new_native_with_env(
        &wasmer_store,
        HostEnvironment {
            memory: LazyInit::default(),
            transform_result: transform_result.clone(),
        },
        set_transform_result,
    );

    let imports = imports! {
        "env" => {
            "__set_transform_result" => set_transform_result_fn_decl,
        }
    };

    let instance = Instance::new(&module, &imports).unwrap();

    let input_ptr = write_bytes_into_guest(&instance, &input_serialized);

    let transform_fn = instance
        .exports
        .get_native_function::<(i32, i32), i32>("__guest_transform")
        .unwrap();


    transform_fn.call(input_ptr.0, input_ptr.1).unwrap();

    let result = read_bytes_from_guest(&transform_result);

    println!("{:#?}", result);
}