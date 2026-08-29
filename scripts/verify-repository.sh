#!/usr/bin/env bash
# Rerunnable local release-readiness checks for the current worktree.
set -euo pipefail

cd "$(dirname "$0")/.."
export CARGO_TERM_COLOR=never

for command in cargo node git; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "ERROR: required command is unavailable: $command" >&2
    exit 1
  fi
done

for script in scripts/*.mjs; do
  node --check "$script"
done
bash -n packaging/macos/build-app.sh scripts/verify-repository.sh

node scripts/validate-site.mjs
node scripts/check-release-version.mjs 0.6.0

package_test="$(mktemp -d)"
trap 'rm -rf "$package_test"' EXIT
printf 'windows-test-artifact' > "$package_test/ShareCursor-Setup-0.6.0.exe"
printf 'macos-test-artifact' > "$package_test/ShareCursor-0.6.0.dmg"
node scripts/generate-release-packages.mjs \
  --version 0.6.0 \
  --windows "$package_test/ShareCursor-Setup-0.6.0.exe" \
  --macos "$package_test/ShareCursor-0.6.0.dmg" \
  --out-dir "$package_test/out"
node - "$package_test" <<'NODE'
const { createHash } = require("node:crypto");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const root = process.argv[2];
const manifest = JSON.parse(readFileSync(join(root, "out", "sharecursor.json"), "utf8"));
if (manifest.version !== "0.6.0" || !/^[a-f0-9]{64}$/.test(manifest.architecture["64bit"].hash)) {
  throw new Error("generated Scoop manifest is invalid");
}
const cask = readFileSync(join(root, "out", "sharecursor.rb"), "utf8");
if (!cask.includes('version "0.6.0"') || !/sha256 "[a-f0-9]{64}"/.test(cask)) {
  throw new Error("generated Homebrew cask is invalid");
}
for (const line of readFileSync(join(root, "out", "ShareCursor-SHA256SUMS.txt"), "utf8").trim().split("\n")) {
  const [expected, file] = line.split(/\s{2}/);
  const actual = createHash("sha256").update(readFileSync(join(root, file))).digest("hex");
  if (actual !== expected) throw new Error(`checksum mismatch for ${file}`);
}
console.log("OK: generated package metadata and checksums");
NODE

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets

git diff --check

tracked_local_tooling="$(git ls-files '.agents/**' AGENTS.md skills-lock.json '.ralph/**')"
if [[ -n "$tracked_local_tooling" ]]; then
  echo "ERROR: local agent or Ralph files remain tracked:" >&2
  printf '%s\n' "$tracked_local_tooling" >&2
  exit 1
fi

stale_files="$(grep -RIlE 'ShareClick|shareclick|autoresearch|AI agent|PROMOTION\.md|SEO\.md|HISTORY\.md|macos-cursor-capture\.md' . \
  --exclude-dir=.git --exclude-dir=.agents --exclude-dir=.ralph --exclude-dir=target \
  --exclude=AGENTS.md --exclude=skills-lock.json --exclude=verify-repository.sh || true)"
if [[ -n "$stale_files" ]]; then
  echo "ERROR: stale brand or internal-artifact references remain:" >&2
  printf '%s\n' "$stale_files" >&2
  exit 1
fi

large_files="$(find . -path './.git' -prune -o -path './.agents' -prune -o -path './.ralph' -prune -o -path './target' -prune -o -type f -size +5M -print)"
if [[ -n "$large_files" ]]; then
  echo "ERROR: worktree contains files larger than 5 MiB:" >&2
  printf '%s\n' "$large_files" >&2
  exit 1
fi

printf '%s\n' "OK: repository release-readiness verification passed"
