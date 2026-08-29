#!/usr/bin/env node

import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const siteDirectory = path.join(repositoryRoot, "site");
const outputPath = path.join(siteDirectory, "llms-full.txt");
const checkOnly = process.argv.includes("--check");

const pages = [
  "index",
  "how-to-share-mouse-keyboard-mac-windows",
  "share-clipboard-between-mac-and-windows",
  "what-is-a-software-kvm",
  "synergy-alternatives",
  "vs-synergy",
  "vs-sharemouse",
  "vs-barrier",
  "vs-mouse-without-borders",
  "vs-universal-control",
  "vs-input-leap",
  "pricing",
  "support",
  "privacy",
];

function contentUpdateDate(sitemap) {
  const dates = [...sitemap.matchAll(/<lastmod>(\d{4}-\d{2}-\d{2})<\/lastmod>/g)].map(
    ([, date]) => date,
  );
  if (dates.length === 0) {
    throw new Error("site/sitemap.xml has no lastmod values");
  }

  const latest = dates.sort().at(-1);
  const [year, month, day] = latest.split("-").map(Number);
  const monthName = new Intl.DateTimeFormat("en-US", {
    month: "long",
    timeZone: "UTC",
  }).format(new Date(Date.UTC(year, month - 1, day)));
  return `${monthName} ${day}, ${year}`;
}

async function generate() {
  const sitemap = await readFile(path.join(siteDirectory, "sitemap.xml"), "utf8");
  const sections = await Promise.all(
    pages.map(async (slug) => {
      const markdown = (await readFile(path.join(siteDirectory, `${slug}.md`), "utf8"))
        .replaceAll("\r\n", "\n")
        .trim();
      return `Source: https://sharecursor.com/${slug}.md\n\n${markdown}`;
    }),
  );

  return [
    "# ShareCursor — full website content",
    "",
    "> Consolidated content from ShareCursor's public Markdown pages for machine readers and offline reference.",
    `> Last content update: ${contentUpdateDate(sitemap)}.`,
    "",
    ...sections.flatMap((section, index) => (index === 0 ? [section] : ["---", "", section])),
    "",
  ].join("\n");
}

const generated = await generate();

if (checkOnly) {
  const current = (await readFile(outputPath, "utf8")).replaceAll("\r\n", "\n");
  if (current !== generated) {
    console.error("ERROR: site/llms-full.txt is stale");
    console.error("Run: node scripts/generate-llms-full.mjs");
    process.exit(1);
  }
  console.log("OK: site/llms-full.txt is current");
} else {
  await writeFile(outputPath, generated, "utf8");
  console.log("OK: generated site/llms-full.txt");
}
