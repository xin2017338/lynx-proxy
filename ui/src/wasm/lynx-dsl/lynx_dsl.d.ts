/* tslint:disable */
/* eslint-disable */
export function eval_program_wasm(program: any, facts: any): boolean;
export function format_dsl_wasm(source: string): string | undefined;
export function init(): void;
export function collect_dsl_syntax_diagnostics(source: string): any;
export function compile_match_expr_wasm(source: string): any;
export function validate_dsl(source: string): any;
export function parse_dsl_program_wasm(source: string): any;
export function validate_dsl_document_wasm(source: string): any;
export function has_dsl_parse_errors(source: string): boolean;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly collect_dsl_syntax_diagnostics: (a: number, b: number) => any;
  readonly compile_match_expr_wasm: (a: number, b: number) => any;
  readonly eval_program_wasm: (a: any, b: any) => number;
  readonly format_dsl_wasm: (a: number, b: number) => [number, number];
  readonly has_dsl_parse_errors: (a: number, b: number) => number;
  readonly init: () => void;
  readonly parse_dsl_program_wasm: (a: number, b: number) => any;
  readonly validate_dsl: (a: number, b: number) => any;
  readonly validate_dsl_document_wasm: (a: number, b: number) => any;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
