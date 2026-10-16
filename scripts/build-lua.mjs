// Compiles src/core to a single Lua file with TypeScriptToLua and copies it
// into the Cyberpunk 2077 CET mod folder.
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = new URL("..", import.meta.url).pathname;
const r = spawnSync(join(root, "node_modules/.bin/tstl"), ["-p", "tsconfig.lua.json"], { cwd: root, stdio: "inherit" });
if (r.status !== 0) process.exit(r.status ?? 1);
const out = join(root, "build/lua/gridlock_core.lua");
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const banner = `-- Gridlock core v${pkg.version} — generated from src/core by TypeScriptToLua. Do not edit by hand.
-- LuaJIT: the simulation has many branches; the default trace cache is too small
-- and thrashes (10x slowdown). Enlarge it when the jit module is available.
if jit and jit.opt and jit.opt.start then pcall(jit.opt.start, "maxtrace=8000", "maxmcode=16384") end
`;
writeFileSync(out, banner + readFileSync(out, "utf8"));
const modDir = join(root, "mods/cyberpunk2077/bin/x64/plugins/cyber_engine_tweaks/mods/gridlock");
mkdirSync(modDir, { recursive: true });
copyFileSync(out, join(modDir, "gridlock_core.lua"));
// Syntax-check the output with every available interpreter.
for (const lua of ["luajit", "lua5.1"]) {
  const chk = spawnSync(lua, ["-e", `assert(loadfile(${JSON.stringify(out)}))`], { encoding: "utf8" });
  if (chk.error) continue;
  if (chk.status !== 0) {
    console.error(`Lua syntax check failed under ${lua}:\n${chk.stderr}`);
    process.exit(1);
  }
}
console.log(`built ${out} (${(readFileSync(out).length / 1024).toFixed(0)} KiB)`);
