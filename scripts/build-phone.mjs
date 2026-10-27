// 2.118.0: TIQR for a phone's browser. Its Rust - the same as the computer's
// app (src-tauri/src/phone.rs) - built for WebAssembly, the browser's half of
// it generated beside the screens (src/phone/pkg), then the site itself
// (dist-phone): `npm run build:phone`.
//
// Needs the WebAssembly target and wasm-bindgen of the version Cargo.lock
// names:
//   rustup target add wasm32-unknown-unknown
//   cargo install wasm-bindgen-cli --version <the wasm-bindgen version in src-tauri/Cargo.lock>
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const run = (cmd, args) => execFileSync(cmd, args, { stdio: "inherit" });

const lock = readFileSync("src-tauri/Cargo.lock", "utf8");
const wanted = /name = "wasm-bindgen"\nversion = "([^"]+)"/.exec(lock)?.[1];
const have = execFileSync("wasm-bindgen", ["--version"], { encoding: "utf8" }).trim().split(" ").pop();
if (wanted && have !== wanted) {
  console.error(`wasm-bindgen ${have} is installed, Cargo.lock wants ${wanted}:\n  cargo install wasm-bindgen-cli --version ${wanted}`);
  process.exit(1);
}

run("cargo", [
  "build",
  "--lib",
  "--profile",
  "phone",
  "--target",
  "wasm32-unknown-unknown",
  "--manifest-path",
  "src-tauri/Cargo.toml",
]);
run("wasm-bindgen", [
  "--target",
  "web",
  "--out-dir",
  "src/phone/pkg",
  "src-tauri/target/wasm32-unknown-unknown/phone/tiqr_manager_lib.wasm",
]);
run("npx", ["vite", "build", "--mode", "phone"]);
