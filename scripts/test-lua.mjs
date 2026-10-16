// Runs the Lua smoke test against every available Lua interpreter.
import { spawnSync } from "node:child_process";
import { join } from "node:path";

const root = new URL("..", import.meta.url).pathname;
const bundle = join(root, "build/lua/gridlock_core.lua");
const interpreters = ["luajit", "lua5.1"];
let ran = 0;
let failed = 0;
for (const lua of interpreters) {
  const probe = spawnSync(lua, ["-v"], { encoding: "utf8" });
  if (probe.error) {
    console.log(`(skipping ${lua}: not installed)`);
    continue;
  }
  console.log(`\n=== ${lua} ===`);
  for (const script of ["tests/lua/smoke.lua", "tests/lua/cet_mock.lua"]) {
    const args = script.endsWith("smoke.lua") ? [join(root, script), bundle] : [join(root, script)];
    const r = spawnSync(lua, args, { stdio: "inherit", cwd: root });
    ran++;
    if (r.status !== 0) failed++;
  }
}
if (ran === 0) {
  console.log("No Lua interpreter found (install luajit or lua5.1).");
  process.exit(1);
}
process.exit(failed > 0 ? 1 : 0);
