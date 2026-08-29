#!/usr/bin/env node

import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const argumentsByName = new Map();
for (let index = 2; index < process.argv.length; index += 2) {
  const name = process.argv[index];
  const value = process.argv[index + 1];
  if (!name?.startsWith("--") || value === undefined) {
    throw new Error("Expected --version, --windows, --macos, and --out-dir arguments");
  }
  argumentsByName.set(name.slice(2), value);
}

const version = argumentsByName.get("version");
const windowsPath = argumentsByName.get("windows");
const macosPath = argumentsByName.get("macos");
const outputDirectory = argumentsByName.get("out-dir");
if (!version || !windowsPath || !macosPath || !outputDirectory) {
  throw new Error("Expected --version, --windows, --macos, and --out-dir arguments");
}
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error(`Invalid release version: ${version}`);
}

const expectedWindowsName = `ShareCursor-Setup-${version}.exe`;
const expectedMacosName = `ShareCursor-${version}.dmg`;
if (path.basename(windowsPath) !== expectedWindowsName) {
  throw new Error(`Expected Windows artifact ${expectedWindowsName}`);
}
if (path.basename(macosPath) !== expectedMacosName) {
  throw new Error(`Expected macOS artifact ${expectedMacosName}`);
}

async function sha256(filePath) {
  return createHash("sha256").update(await readFile(filePath)).digest("hex");
}

async function render(templatePath, replacements) {
  let content = await readFile(path.join(repositoryRoot, templatePath), "utf8");
  for (const [token, value] of Object.entries(replacements)) {
    content = content.replaceAll(`__${token}__`, value);
  }
  const unresolved = content.match(/__[A-Z0-9_]+__/g);
  if (unresolved) {
    throw new Error(`Unresolved template tokens in ${templatePath}: ${unresolved.join(", ")}`);
  }
  return content.replaceAll("\r\n", "\n");
}

const [windowsHash, macosHash] = await Promise.all([
  sha256(windowsPath),
  sha256(macosPath),
]);
const replacements = {
  VERSION: version,
  WINDOWS_SHA256: windowsHash,
  MACOS_SHA256: macosHash,
};

await mkdir(outputDirectory, { recursive: true });
await Promise.all([
  writeFile(
    path.join(outputDirectory, "sharecursor.json"),
    await render("packaging/scoop/sharecursor.json.template", replacements),
    "utf8",
  ),
  writeFile(
    path.join(outputDirectory, "sharecursor.rb"),
    await render("packaging/homebrew/sharecursor.rb.template", replacements),
    "utf8",
  ),
  writeFile(
    path.join(outputDirectory, "ShareCursor-SHA256SUMS.txt"),
    `${windowsHash}  ${expectedWindowsName}\n${macosHash}  ${expectedMacosName}\n`,
    "utf8",
  ),
]);

console.log(`OK: generated release package metadata for ${version}`);
