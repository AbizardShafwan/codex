import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { chmod, copyFile, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";

async function fixture(t) {
  const root = await mkdtemp(path.join(os.tmpdir(), "memyr-launcher-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const bin = path.join(root, "codex-cli", "bin");
  await mkdir(bin, { recursive: true });
  await writeFile(path.join(root, "codex-cli", "package.json"), '{"type":"module"}');
  const launcher = path.join(bin, "memyr.js");
  await copyFile(new URL("../bin/memyr.js", import.meta.url), launcher);
  return { root, launcher };
}

async function native(root, program) {
  const binary = path.join(root, "codex-rs", "target", "debug", "memyr");
  await mkdir(path.dirname(binary), { recursive: true });
  await writeFile(binary, `#!/usr/bin/env node\n${program}\n`);
  await chmod(binary, 0o755);
}

function start(launcher, args = []) {
  const child = spawn(process.execPath, [launcher, ...args], {
    env: { ...process.env, MEMYR_LAUNCHER_TEST: "inherited" },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let stdout = "";
  let stderr = "";
  child.stdout.on("data", (data) => (stdout += data));
  child.stderr.on("data", (data) => (stderr += data));
  const done = once(child, "close").then(([code, signal]) => ({ code, signal, stdout, stderr }));
  return { child, done };
}

test("forwards arguments, environment and native exit code", { skip: process.platform === "win32" }, async (t) => {
  const { root, launcher } = await fixture(t);
  await native(root, 'console.log(JSON.stringify({args:process.argv.slice(2),marker:process.env.MEMYR_LAUNCHER_TEST})); process.exit(7);');
  const result = await start(launcher, ["exec", "two words", "--model", "test-model"]).done;
  assert.equal(result.code, 7);
  assert.equal(result.signal, null);
  assert.deepEqual(JSON.parse(result.stdout), {
    args: ["exec", "two words", "--model", "test-model"], marker: "inherited",
  });
});

test("missing Memyr does not launch a legacy codex binary", async (t) => {
  const { root, launcher } = await fixture(t);
  const legacy = path.join(root, "codex-rs", "target", "debug", "codex");
  await mkdir(path.dirname(legacy), { recursive: true });
  await writeFile(legacy, "legacy executable must not run");
  const result = await start(launcher).done;
  assert.equal(result.code, 1);
  assert.match(result.stderr, /Memyr executable not found/);
  assert.match(result.stderr, /--bin memyr/);
});

test("bundled Memyr takes precedence over a checkout build", { skip: process.platform !== "linux" || process.arch !== "x64" }, async (t) => {
  const { root, launcher } = await fixture(t);
  await native(root, 'console.log("checkout");');
  const binary = path.join(root, "codex-cli/vendor/x86_64-unknown-linux-musl/bin/memyr");
  await mkdir(path.dirname(binary), { recursive: true });
  await writeFile(binary, '#!/usr/bin/env node\nconsole.log("bundled");\n');
  await chmod(binary, 0o755);
  const result = await start(launcher).done;
  assert.equal(result.code, 0);
  assert.equal(result.stdout.trim(), "bundled");
});

test("forwards SIGTERM and mirrors native signal termination", { skip: process.platform === "win32", timeout: 10000 }, async (t) => {
  const { root, launcher } = await fixture(t);
  await native(root, 'console.log("ready"); setInterval(() => {}, 1000);');
  const { child, done } = start(launcher);
  t.after(() => { if (child.exitCode === null && child.signalCode === null) child.kill("SIGKILL"); });
  await once(child.stdout, "data");
  child.kill("SIGTERM");
  const result = await done;
  assert.equal(result.signal, "SIGTERM");
  assert.equal(result.code, null);
  assert.match(result.stdout, /ready/);
});
