miniquad_add_plugin({
    register_plugin: function (importObject) {
        importObject.env.storage_get = function (key_ptr, key_len, buf_ptr, buf_len) {
            const key = new TextDecoder().decode(new Uint8Array(wasm_memory.buffer, key_ptr, key_len));
            const value = localStorage.getItem(key);
            if (value === null) return -1;
            const encoded = new TextEncoder().encode(value);
            if (buf_ptr !== 0 && encoded.length <= buf_len) {
                new Uint8Array(wasm_memory.buffer, buf_ptr, encoded.length).set(encoded);
            }
            return encoded.length;
        };
        importObject.env.storage_set = function (key_ptr, key_len, val_ptr, val_len) {
            const key = new TextDecoder().decode(new Uint8Array(wasm_memory.buffer, key_ptr, key_len));
            const val = new TextDecoder().decode(new Uint8Array(wasm_memory.buffer, val_ptr, val_len));
            localStorage.setItem(key, val);
        };
    },
    name: "fetris_storage",
    version: "0.1.0"
});
