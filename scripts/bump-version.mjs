#!/usr/bin/env node
// Set the app version everywhere it lives.
//   node scripts/bump-version.mjs patch|minor|major|1.2.3
// Prints the new version.
import { readFileSync, writeFileSync } from "node:fs";

const arg = process.argv[2] ?? "patch";
const cargo = readFileSync("Cargo.toml", "utf8");
const current = cargo.match(/\[workspace\.package\][^[]*?version\s*=\s*"([^"]+)"/s)?.[1];
if (!current) throw new Error("no version in Cargo.toml [workspace.package]");

let next;
if (/^\d+\.\d+\.\d+$/.test(arg)) next = arg;
else {
  let [a, b, c] = current.split(".").map(Number);
  if (arg === "major") [a, b, c] = [a + 1, 0, 0];
  else if (arg === "minor") [b, c] = [b + 1, 0];
  else c += 1;
  next = `${a}.${b}.${c}`;
}

writeFileSync("Cargo.toml", cargo.replace(/(\[workspace\.package\][^[]*?version\s*=\s*")[^"]+(")/s, `$1${next}$2`));
for (const f of ["src-tauri/tauri.conf.json", "package.json"]) {
  const j = JSON.parse(readFileSync(f, "utf8"));
  j.version = next;
  writeFileSync(f, JSON.stringify(j, null, 2) + "\n");
}
const lock = JSON.parse(readFileSync("package-lock.json", "utf8"));
lock.version = next;
if (lock.packages?.[""]) lock.packages[""].version = next;
writeFileSync("package-lock.json", JSON.stringify(lock, null, 2) + "\n");
console.log(next);
