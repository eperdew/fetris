/* tslint:disable */
/* eslint-disable */

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly main: (a: number, b: number) => number;
    readonly allocate_vec_u8: (a: number) => number;
    readonly crate_version: () => number;
    readonly file_loaded: (a: number) => void;
    readonly focus: (a: number) => void;
    readonly frame: () => void;
    readonly key_down: (a: number, b: number, c: number) => void;
    readonly key_press: (a: number) => void;
    readonly key_up: (a: number, b: number) => void;
    readonly mouse_down: (a: number, b: number, c: number) => void;
    readonly mouse_move: (a: number, b: number) => void;
    readonly mouse_up: (a: number, b: number, c: number) => void;
    readonly mouse_wheel: (a: number, b: number) => void;
    readonly on_clipboard_paste: (a: number, b: number) => void;
    readonly on_file_dropped: (a: number, b: number, c: number, d: number) => void;
    readonly on_files_dropped_finish: () => void;
    readonly on_files_dropped_start: () => void;
    readonly raw_mouse_move: (a: number, b: number) => void;
    readonly resize: (a: number, b: number) => void;
    readonly touch: (a: number, b: number, c: number, d: number) => void;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
