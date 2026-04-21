#[cfg(target_arch = "wasm32")]
mod imp {
    unsafe extern "C" {
        fn storage_get(key_ptr: *const u8, key_len: u32, buf_ptr: *mut u8, buf_len: u32) -> i32;
        fn storage_set(key_ptr: *const u8, key_len: u32, val_ptr: *const u8, val_len: u32);
    }

    pub struct Storage;

    impl Storage {
        pub fn new() -> Self {
            Storage
        }

        pub fn get(&self, key: &str) -> Option<String> {
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

        pub fn set(&mut self, key: &str, value: &str) {
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
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::collections::HashMap;
    use std::path::PathBuf;

    pub struct Storage {
        path: PathBuf,
        map: HashMap<String, String>,
    }

    impl Storage {
        pub fn new() -> Self {
            let path = Self::store_path();
            let map = std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            Storage { path, map }
        }

        fn store_path() -> PathBuf {
            let base = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            base.join(".config").join("fetris").join("storage.json")
        }

        pub fn get(&self, key: &str) -> Option<String> {
            self.map.get(key).cloned()
        }

        pub fn set(&mut self, key: &str, value: &str) {
            self.map.insert(key.to_string(), value.to_string());
            if let Some(parent) = self.path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string(&self.map) {
                let _ = std::fs::write(&self.path, json);
            }
        }
    }
}

pub use imp::Storage;
