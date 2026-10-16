// Regenerates the tables inside docs/*.md between <!-- gen:NAME --> markers.
import { build } from "esbuild";
import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { execFileSync } from "node:child_process";

const root = new URL("..", import.meta.url).pathname;
const out = join(root, "build/docgen.cjs");
await build({ entryPoints: [join(root, "scripts/docgen/tables.ts")], bundle: true, platform: "node", outfile: out, logLevel: "warning" });
const files = ["README.md", ...readdirSync(join(root, "docs")).map((f) => join("docs", f))].filter((f) => f.endsWith(".md"));
for (const f of files) {
  const path = join(root, f);
  let text = readFileSync(path, "utf8");
  const before = text;
  text = text.replace(/<!-- gen:(\w+) -->[\s\S]*?<!-- \/gen -->/g, (_m, name) => {
    const table = execFileSync(process.execPath, [out, name], { encoding: "utf8" });
    return `<!-- gen:${name} -->\n${table.trim()}\n<!-- /gen -->`;
  });
  if (text !== before) {
    writeFileSync(path, text);
    console.log(`updated ${f}`);
  }
}
