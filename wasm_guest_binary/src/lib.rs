use rkyv::{AlignedVec, archived_root, Deserialize};
use runner_base::SharedStruct;

#[cfg(target_arch = "wasm32")]
#[no_mangle]
#[inline(always)]
pub extern "C" fn __alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
#[inline(always)]
pub extern "C" fn __free(ptr: *mut u8, size: usize) -> i32 {
    let data = unsafe { Vec::from_raw_parts(ptr, size, size) };
    std::mem::drop(data);
    0
}

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn __set_transform_result(bytes_ptr: i32, bytes_ptr_len: i32);
}

#[no_mangle]
pub fn __guest_transform(ptr: *const u8, ptr_len: i32) -> i32 {

    let raw_ast_serialized_bytes = unsafe { std::slice::from_raw_parts(ptr, ptr_len.try_into().unwrap()) };
    let mut aligned_vec = AlignedVec::with_capacity(raw_ast_serialized_bytes.len());
    aligned_vec.extend_from_slice(raw_ast_serialized_bytes);

    let deserialized = unsafe { archived_root::<SharedStruct>(&aligned_vec[..]) };
    let shared_struct_original: SharedStruct = deserialized.deserialize(&mut rkyv::Infallible).unwrap();

    let mut l = vec!["updated_5".to_string()];
    l.extend_from_slice(&shared_struct_original.list[1..]);

    let mut l2 = vec![15];
    l2.extend_from_slice(&shared_struct_original.other_list[2..]);

    let result = SharedStruct {
        name: shared_struct_original.name + "_updated",
        list: l,
        other_list: l2
    };


    let result_serialized = rkyv::to_bytes::<_, 512>(&result).unwrap();

    #[cfg(target_arch = "wasm32")]
    unsafe {
        __set_transform_result(result_serialized.as_ptr() as _, result_serialized.len().try_into().unwrap());
    }
    0
}
