#[cfg(target_arch = "wasm32")]
mod imp {
    extern "C" {
        fn storage_get(key_ptr: *const u8, key_len: u32, buf_ptr: *mut u8, buf_len: u32) -> i32;
        fn storage_set(key_ptr: *const u8, key_len: u32, val_ptr: *const u8, val_len: u32);
    }

    pub fn get(key: &str) -> Option<String> {
        unsafe {
            let len = storage_get(key.as_ptr(), key.len() as u32, std::ptr::null_mut(), 0);
            if len < 0 {
                return None;
            }
            let mut buf = vec![0u8; len as usize];
            storage_get(key.as_ptr(), key.len() as u32, buf.as_mut_ptr(), len as u32);
            String::from_utf8(buf).ok()
        }
    }

    pub fn set(key: &str, value: &str) {
        unsafe {
            storage_set(
                key.as_ptr(),
                key.len() as u32,
                value.as_ptr(),
                value.len() as u32,
            );
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::collections::HashMap;
    use std::sync::Mutex;

    static STORE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

    pub fn get(key: &str) -> Option<String> {
        STORE.lock().unwrap().as_ref()?.get(key).cloned()
    }

    pub fn set(key: &str, value: &str) {
        STORE
            .lock()
            .unwrap()
            .get_or_insert_with(HashMap::new)
            .insert(key.to_string(), value.to_string());
    }
}

pub use imp::{get, set};
