// 2.118.0: what `npm run build:phone` generates in src/phone/pkg (wasm-bindgen,
// from src-tauri/src/phone.rs) - declared here too, so the app's own type
// check passes on a computer that never built it. Once built, the generated
// declarations are the ones read.
declare module "*/pkg/tiqr_manager_lib.js" {
  export default function init(options?: { module_or_path?: string | URL }): Promise<unknown>;
  export function start(
    emit: (event: string, payload: string) => void,
    http: (method: string, url: string, headers: string, body: Uint8Array | undefined, timeoutMs: number) => unknown,
  ): Promise<void>;
  export function invoke(cmd: string, args: string): Promise<unknown>;
}
