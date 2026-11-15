// Builds the Rust bridge (wasm/) and generates its JavaScript glue into
// src/wasm-pkg/. Needs: Rust with the wasm32-unknown-unknown target, and
// wasm-bindgen-cli at the same version as the wasm-bindgen crate:
//   rustup target add wasm32-unknown-unknown
//   cargo install wasm-bindgen-cli --version 0.2.130 --locked
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const web = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const repo = path.dirname(web);
const run = (cmd, args) => execFileSync(cmd, args, { cwd: repo, stdio: "inherit" });

run("cargo", ["build", "-q", "-p", "skyglow-wasm", "--release", "--target", "wasm32-unknown-unknown"]);
run("wasm-bindgen", [
  path.join(repo, "target", "wasm32-unknown-unknown", "release", "skyglow_wasm.wasm"),
  "--target", "web",
  "--out-dir", path.join(web, "src", "wasm-pkg"),
]);
console.log("wasm: built web/src/wasm-pkg");
