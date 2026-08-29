#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { readFile, readdir, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const siteDirectory = path.join(repositoryRoot, "site");
const errors = [];
const assert = (condition, message) => {
  if (!condition) errors.push(message);
};
const readSite = (file) => readFile(path.join(siteDirectory, file), "utf8");
const exists = async (file) => {
  try {
    return (await stat(path.join(siteDirectory, file))).isFile();
  } catch {
    return false;
  }
};

function attribute(html, tagPattern, attributeName) {
  const tag = html.match(tagPattern)?.[0];
  return tag?.match(new RegExp(`${attributeName}=["']([^"']+)["']`, "i"))?.[1];
}

function localTarget(reference, sourceFile) {
  const decoded = reference.replaceAll("&amp;", "&").split(/[?#]/, 1)[0];
  if (!decoded || decoded.startsWith("#") || /^(?:mailto|tel|javascript|data):/i.test(decoded)) {
    return null;
  }

  if (/^https?:\/\//i.test(decoded)) {
    const url = new URL(decoded);
    if (url.origin !== "https://sharecursor.com") return null;
    const relative = decodeURIComponent(url.pathname.replace(/^\//, ""));
    return relative === "" || relative.endsWith("/") ? path.join(relative, "index.html") : relative;
  }

  if (decoded.startsWith("//")) return null;
  const relative = decoded.startsWith("/")
    ? decoded.slice(1)
    : path.join(path.dirname(sourceFile), decodeURIComponent(decoded));
  const normalized = path.normalize(relative || "index.html");
  return normalized === "." || normalized.endsWith(path.sep)
    ? path.join(normalized, "index.html")
    : normalized;
}

function pngDimensions(buffer) {
  const signature = "89504e470d0a1a0a";
  if (buffer.subarray(0, 8).toString("hex") !== signature) return null;
  return [buffer.readUInt32BE(16), buffer.readUInt32BE(20)];
}

const sitemap = await readSite("sitemap.xml");
const sitemapUrls = [...sitemap.matchAll(/<loc>(https:\/\/sharecursor\.com\/[^<]*)<\/loc>/g)].map(([, url]) => url);
const lastModified = [...sitemap.matchAll(/<lastmod>([^<]+)<\/lastmod>/g)].map(([, date]) => date);
assert(sitemapUrls.length > 0, "sitemap.xml has no ShareCursor URLs");
assert(new Set(sitemapUrls).size === sitemapUrls.length, "sitemap.xml contains duplicate URLs");
assert(lastModified.length === sitemapUrls.length, "every sitemap URL must have one lastmod");
for (const date of lastModified) {
  assert(/^\d{4}-\d{2}-\d{2}$/.test(date) && !Number.isNaN(Date.parse(`${date}T00:00:00Z`)), `invalid sitemap lastmod: ${date}`);
}
assert(!/<(?:changefreq|priority)>/.test(sitemap), "sitemap.xml should not contain changefreq or priority hints");

const htmlFiles = (await readdir(siteDirectory)).filter((file) => file.endsWith(".html"));
const indexedFiles = new Set();
for (const urlString of sitemapUrls) {
  const url = new URL(urlString);
  const file = url.pathname === "/" ? "index.html" : decodeURIComponent(url.pathname.slice(1));
  indexedFiles.add(file);
  assert(await exists(file), `sitemap target is missing: ${file}`);
  if (!(await exists(file))) continue;

  const html = await readSite(file);
  const canonical = attribute(html, /<link\b[^>]*rel=["']canonical["'][^>]*>/i, "href");
  const alternateTag = html.match(/<link\b[^>]*rel=["']alternate["'][^>]*type=["']text\/markdown["'][^>]*>/i)?.[0];
  const alternate = alternateTag?.match(/href=["']([^"']+)["']/i)?.[1];
  assert(canonical === urlString, `${file}: canonical does not match sitemap URL`);
  assert((html.match(/<title>[^<]+<\/title>/gi) || []).length === 1, `${file}: expected one non-empty title`);
  assert(/<meta\b[^>]*name=["']description["'][^>]*content=["'][^"']+["']/i.test(html), `${file}: missing meta description`);
  assert((html.match(/<h1\b/gi) || []).length === 1, `${file}: expected exactly one h1`);
  assert(alternate, `${file}: missing Markdown alternate`);
  if (alternate) assert(await exists(localTarget(alternate, file)), `${file}: Markdown alternate is missing: ${alternate}`);

  for (const property of ["og:type", "og:site_name", "og:title", "og:description", "og:url", "og:image", "og:image:alt"]) {
    assert(new RegExp(`<meta\\b[^>]*property=["']${property}["'][^>]*content=["'][^"']+["']`, "i").test(html), `${file}: missing ${property}`);
  }
  assert(/<meta\b[^>]*name=["']twitter:card["'][^>]*content=["']summary_large_image["']/i.test(html), `${file}: missing Twitter card`);
  assert(/<meta\b[^>]*name=["']twitter:image["'][^>]*content=["'][^"']+["']/i.test(html), `${file}: missing Twitter image`);
  assert(html.includes('data-website-id="19332134-ef62-4f1c-be25-531b327c4223"'), `${file}: missing configured Umami analytics`);
  assert(html.includes('href="support.html"') && html.includes('href="privacy.html"'), `${file}: missing support/privacy footer links`);

  const jsonLdBlocks = [...html.matchAll(/<script\b[^>]*type=["']application\/ld\+json["'][^>]*>([\s\S]*?)<\/script>/gi)];
  assert(jsonLdBlocks.length > 0, `${file}: missing JSON-LD`);
  for (const [, rawJson] of jsonLdBlocks) {
    try {
      JSON.parse(rawJson);
    } catch (error) {
      errors.push(`${file}: invalid JSON-LD: ${error.message}`);
    }
  }
}

for (const file of htmlFiles) {
  if (file === "google7a315636abafd645.html") continue;
  const html = await readSite(file);
  assert(/^<!doctype html>/i.test(html), `${file}: missing doctype`);
  assert(/<html\b[^>]*lang=["']en["']/i.test(html), `${file}: missing English language declaration`);
  assert(/<\/html>\s*$/i.test(html), `${file}: missing closing html tag`);
  assert(!/fonts\.googleapis\.com|fonts\.gstatic\.com/i.test(html), `${file}: remote Google Font reference remains`);
  assert(!/<meta\b[^>]*name=["']keywords["']/i.test(html), `${file}: obsolete meta keywords remain`);

  for (const match of html.matchAll(/<(?:a|link|script|img)\b[^>]*(?:href|src)=["']([^"']+)["'][^>]*>/gi)) {
    const target = localTarget(match[1], file);
    if (!target) continue;
    const safeTarget = path.normalize(target);
    assert(!safeTarget.startsWith(`..${path.sep}`) && !path.isAbsolute(safeTarget), `${file}: local link escapes site/: ${match[1]}`);
    if (!safeTarget.startsWith("..") && !path.isAbsolute(safeTarget)) {
      assert(await exists(safeTarget), `${file}: missing local target ${match[1]} -> ${safeTarget}`);
    }
  }
}

const canonicalHtmlFiles = new Set(htmlFiles.filter((file) => !["404.html", "google7a315636abafd645.html"].includes(file)));
assert(canonicalHtmlFiles.size === indexedFiles.size, "sitemap and canonical HTML page counts differ");
for (const file of canonicalHtmlFiles) assert(indexedFiles.has(file), `${file}: canonical page is absent from sitemap`);

const markdownFiles = (await readdir(siteDirectory)).filter((file) => file.endsWith(".md")).sort();
const llms = await readSite("llms.txt");
for (const file of markdownFiles) {
  assert(llms.includes(`https://sharecursor.com/${file}`), `llms.txt does not reference ${file}`);
}
execFileSync(process.execPath, [path.join(repositoryRoot, "scripts", "generate-llms-full.mjs"), "--check"], {
  cwd: repositoryRoot,
  stdio: "inherit",
});

const manifest = JSON.parse(await readSite("manifest.webmanifest"));
for (const icon of manifest.icons || []) {
  assert(await exists(icon.src), `manifest icon is missing: ${icon.src}`);
  const dimensions = icon.sizes?.match(/^(\d+)x(\d+)$/);
  if (dimensions && icon.type === "image/png") {
    const actual = pngDimensions(await readFile(path.join(siteDirectory, icon.src)));
    assert(actual?.[0] === Number(dimensions[1]) && actual?.[1] === Number(dimensions[2]), `${icon.src}: PNG dimensions do not match manifest`);
  }
}

const robots = await readSite("robots.txt");
assert(robots === "User-agent: *\nAllow: /\n\nSitemap: https://sharecursor.com/sitemap.xml\n", "robots.txt is not the expected minimal policy");
assert((await readSite("CNAME")).trim() === "sharecursor.com", "CNAME is not sharecursor.com");

if (errors.length > 0) {
  for (const error of errors) console.error(`ERROR: ${error}`);
  console.error(`ERROR: site validation failed with ${errors.length} issue(s)`);
  process.exit(1);
}
console.log(`OK: validated ${indexedFiles.size} indexed pages, ${markdownFiles.length} Markdown sources, local links, schema, manifest, robots, and sitemap`);
