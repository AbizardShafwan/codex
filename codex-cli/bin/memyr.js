#!/usr/bin/env node
// Launch only a native Memyr build from this checkout or its bundled vendor tree.
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = fileURLToPath(new URL("../", import.meta.url));
const executable = process.platform === "win32" ? "memyr.exe" : "memyr";
const targets = {
  "linux-x64": "x86_64-unknown-linux-musl",
  "linux-arm64": "aarch64-unknown-linux-musl",
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
  "win32-arm64": "aarch64-pc-windows-msvc",
};
const target = targets[`${process.platform}-${process.arch}`];
const candidates = [
  ...(target
    ? [path.join(packageRoot, "vendor", target, "bin", executable)]
    : []),
  path.join(packageRoot, "..", "codex-rs", "target", "release", executable),
  path.join(packageRoot, "..", "codex-rs", "target", "debug", executable),
];
const binary = candidates.find((candidate) => existsSync(candidate));
if (!binary) {
  console.error(
    "Memyr executable not found. Build it from codex-rs with: cargo build --locked -p codex-cli --bin memyr",
  );
  process.exit(1);
}

const child = spawn(binary, process.argv.slice(2), { stdio: "inherit" });
const handlers = new Map();
for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"]) {
  const handler = () => {
    if (!child.killed) {
      try {
        child.kill(signal);
      } catch {
        // The child may have exited before the signal was forwarded.
      }
    }
  };
  handlers.set(signal, handler);
  process.on(signal, handler);
}
child.on("error", (error) => {
  console.error(`Unable to start Memyr: ${error.message}`);
  process.exit(1);
});
child.on("exit", (code, signal) => {
  for (const [name, handler] of handlers) process.off(name, handler);
  if (signal) process.kill(process.pid, signal);
  else process.exit(code ?? 1);
});
