// Bundles the browser sandbox (core + three.js) into a single self-contained HTML file.
//   dist/gridlock.html          full document, open directly in a browser
//   dist/gridlock.artifact.html body fragment (no doctype/head) for hosts that add their own
import { build } from "esbuild";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = new URL("..", import.meta.url).pathname;
const dev = process.argv.includes("--dev");
const result = await build({
  entryPoints: [join(root, "src/sandbox/main.ts")],
  bundle: true,
  minify: !dev,
  format: "iife",
  target: "es2020",
  write: false,
  legalComments: "none",
  logLevel: "warning",
});
const js = result.outputFiles[0].text.replace(/<\/script/gi, "<\\/script");
const css = readFileSync(join(root, "src/sandbox/style.css"), "utf8");
const page = readFileSync(join(root, "src/sandbox/page.html"), "utf8");
const fragment = page.replace("/*CSS*/", () => css).replace("/*JS*/", () => js);
const doc = `<!doctype html>\n<html lang="en">\n<head>\n<meta charset="utf-8">\n<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">\n</head>\n<body>\n${fragment}\n</body>\n</html>\n`;
mkdirSync(join(root, "dist"), { recursive: true });
writeFileSync(join(root, "dist/gridlock.html"), doc);
writeFileSync(join(root, "dist/gridlock.artifact.html"), fragment);
console.log(`dist/gridlock.html ${(doc.length / 1024).toFixed(0)} KiB (js ${(js.length / 1024).toFixed(0)} KiB)`);
