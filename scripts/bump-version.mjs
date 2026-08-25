#!/usr/bin/env node
// 同步三处版本号：package.json / src-tauri/tauri.conf.json / src-tauri/Cargo.toml
// 用法：node scripts/bump-version.mjs <x.y.z | patch | minor | major>
import { readFileSync, writeFileSync } from "node:fs";

const arg = process.argv[2];
if (!arg) {
  console.error("用法: node scripts/bump-version.mjs <x.y.z|patch|minor|major>");
  process.exit(1);
}

const pkg = JSON.parse(readFileSync("package.json", "utf8"));
const cur = pkg.version;
const [maj, min, pat] = cur.split(".").map(Number);

let next;
if (arg === "major") next = `${maj + 1}.0.0`;
else if (arg === "minor") next = `${maj}.${min + 1}.0`;
else if (arg === "patch") next = `${maj}.${min}.${pat + 1}`;
else if (/^\d+\.\d+\.\d+$/.test(arg)) next = arg;
else {
  console.error(`无法识别的版本号: ${arg}`);
  process.exit(1);
}

pkg.version = next;
writeFileSync("package.json", JSON.stringify(pkg, null, 2) + "\n");

const confPath = "src-tauri/tauri.conf.json";
const conf = JSON.parse(readFileSync(confPath, "utf8"));
conf.version = next;
writeFileSync(confPath, JSON.stringify(conf, null, 2) + "\n");

const cargoPath = "src-tauri/Cargo.toml";
const cargo = readFileSync(cargoPath, "utf8");
const updated = cargo.replace(/^version = ".*"$/m, `version = "${next}"`);
if (updated === cargo) {
  console.error("Cargo.toml 里没找到 [package] 的 version 行");
  process.exit(1);
}
writeFileSync(cargoPath, updated);

console.log(`版本号 ${cur} -> ${next}`);
console.log(`提交后记得打 tag: git tag v${next}`);
