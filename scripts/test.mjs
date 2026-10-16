// Bundles every tests/**/*.test.ts with esbuild and runs them with node:test.
import { build } from "esbuild";
import { readdirSync, mkdirSync, rmSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const root = new URL("..", import.meta.url).pathname;
const testDir = join(root, "tests");
const outDir = join(root, "build", "tests");
rmSync(outDir, { recursive: true, force: true });
mkdirSync(outDir, { recursive: true });
const filter = process.argv[2];
const files = readdirSync(testDir).filter((f) => f.endsWith(".test.ts") && (filter === undefined || f.includes(filter)));
await build({
  entryPoints: files.map((f) => join(testDir, f)),
  outdir: outDir,
  bundle: true,
  platform: "node",
  format: "esm",
  sourcemap: "inline",
  logLevel: "warning",
});
const outs = files.map((f) => join(outDir, f.replace(/\.ts$/, ".js")));
const r = spawnSync(process.execPath, ["--enable-source-maps", "--test", ...outs], { stdio: "inherit" });
process.exit(r.status ?? 1);
