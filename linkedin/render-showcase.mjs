import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import path from "node:path";

const require = createRequire(import.meta.url);
const { chromium } = require("playwright");
const dir = path.dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Z]:)/, "$1"));
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1600, height: 900 }, deviceScaleFactor: 1 });
const file = pathToFileURL(path.join(dir, "showcase.html")).href;

for (const [slide, name] of [
  ["cover", "01-ventus-cover.png"],
  ["product", "02-product-experience.png"],
  ["engineering", "03-engineering-architecture.png"],
  ["features", "04-core-capabilities.png"],
]) {
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto(`${file}?slide=${slide}`, { waitUntil: "networkidle" });
  await page.screenshot({ path: path.join(dir, name) });
}

await page.setViewportSize({ width: 1200, height: 1200 });
await page.goto(`${file}?slide=thumb`, { waitUntil: "networkidle" });
await page.screenshot({ path: path.join(dir, "05-linkedin-thumbnail.png") });
await browser.close();
