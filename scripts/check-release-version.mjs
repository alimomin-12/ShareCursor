#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (filePath) => readFile(path.join(repositoryRoot, filePath), "utf8");
const expected = process.argv[2];

const [cargo, lock, installer, homepage, changelog, macBuild] = await Promise.all([
  read("Cargo.toml"),
  read("Cargo.lock"),
  read("packaging/windows/sharecursor.iss"),
  read("site/index.html"),
  read("CHANGELOG.md"),
  read("packaging/macos/build-app.sh"),
]);

const workspaceVersion = cargo.match(/\[workspace\.package\][\s\S]*?\nversion = "([^"]+)"/)?.[1];
if (!workspaceVersion) {
  throw new Error("Could not read workspace version from Cargo.toml");
}
if (expected && workspaceVersion !== expected) {
  throw new Error(`Tag/input version ${expected} does not match Cargo.toml ${workspaceVersion}`);
}
if (!/^\d+\.\d+\.\d+$/.test(workspaceVersion)) {
  throw new Error(`Stable workspace version must be numeric MAJOR.MINOR.PATCH: ${workspaceVersion}`);
}

const checks = [
  ["Cargo.lock sharecursor", new RegExp(`name = "sharecursor"\\nversion = "${workspaceVersion.replaceAll(".", "\\.")}"`), lock],
  ["Cargo.lock sharecursor-protocol", new RegExp(`name = "sharecursor-protocol"\\nversion = "${workspaceVersion.replaceAll(".", "\\.")}"`), lock],
  ["Windows installer", new RegExp(`#define MyAppVersion "${workspaceVersion.replaceAll(".", "\\.")}"`), installer],
  ["website SoftwareApplication schema", new RegExp(`"softwareVersion": "${workspaceVersion.replaceAll(".", "\\.")}"`), homepage],
  ["changelog release heading", new RegExp(`^## \\[${workspaceVersion.replaceAll(".", "\\.")}\\] - \\d{4}-\\d{2}-\\d{2}$`, "m"), changelog],
  ["permanent macOS bundle identifier", /<string>com\.sharecursor\.app<\/string>/, macBuild],
  ["numeric macOS bundle version", /CFBundleVersion<\/key><string>\$BUNDLE_VERSION<\/string>/, macBuild],
];
for (const [label, pattern, content] of checks) {
  if (!pattern.test(content)) {
    throw new Error(`${label} does not match release version ${workspaceVersion}`);
  }
}

console.log(`OK: release version ${workspaceVersion} is consistent`);
