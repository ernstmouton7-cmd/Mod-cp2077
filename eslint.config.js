// Lint rules exist mainly to keep src/core portable to Lua (TypeScriptToLua):
// in Lua, 0 and "" are truthy, so implicit number/string truthiness is banned.
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["build/**", "dist/**", "node_modules/**", "mods/**"] },
  {
    files: ["src/**/*.ts", "tests/**/*.ts"],
    languageOptions: {
      parser: tseslint.parser,
      parserOptions: { project: "./tsconfig.json" },
    },
    plugins: { "@typescript-eslint": tseslint.plugin },
    rules: {
      "@typescript-eslint/strict-boolean-expressions": [
        "error",
        { allowString: false, allowNumber: false, allowNullableObject: true, allowNullableBoolean: true },
      ],
      "@typescript-eslint/no-floating-promises": "off",
    },
  },
  {
    files: ["src/core/**/*.ts"],
    rules: {
      "no-restricted-syntax": [
        "error",
        { selector: "ForInStatement", message: "for..in is not portable to Lua; iterate explicitly." },
        { selector: "BinaryExpression[operator='%']", message: "Use mod()/fmod() from math/scalar: % differs between JS and Lua for negatives." },
        { selector: "UnaryExpression[operator='delete']", message: "delete is not portable to Lua arrays." },
        { selector: "BinaryExpression[operator=/^(<<|>>|>>>|&|\\||\\^)$/]", message: "Bitwise ops are not portable to LuaJIT/5.1 without bit lib." },
      ],
      "no-restricted-globals": ["error", "window", "document", "console", "performance", "setTimeout", "setInterval", "requestAnimationFrame"],
    },
  },
);
