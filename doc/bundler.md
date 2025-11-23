## Bundler

This document describes two complementary, implemented features:

- A safe AST rewrite that replaces selected `require("...")` calls with vendor-specific loader calls (`--replace`).
- A vendor include mode to inline specific vendor roots while honoring excludes and prior rewrites (`--vendor`).

Source code remains standard LuaRocks-style requires for full IDE support; the bundle adapts to engine/runtime realities.

### Source layout assumptions

- First-party application code lives under `lua/` and is bundled.
- Vendor runtime modules live under `vendor/lua/` and are NOT bundled.
- Requires use dotted names (`.`) and may resolve to `init.lua`.

### CLI

```bash
luapack bundle lua/main.lua \
  --lua=5.1 \
  --path="lua/?.lua" \
  --path="lua/?/init.lua" \
  --prelude="prelude1.lua" \
  --prelude="prelude2.lua" \
  --replace="match=prefix,old=require,new=bar_require,prefix=bar." \
  --replace="match=prefix,old=require,new=evorts_require,prefix=evorts." \
  --vendor="vendor/lua/?.lua,exclude=name:lib1,exclude=name:lib2" \
  --vendor="vendor/lua/?/init.lua,exclude=name:lib1,exclude=name:lib2" \
  --output=dist/gui_overwatch.lua
```

- Use multiple `--path` flags for first-party bundle roots (Lua style kept IDE-friendly).
- Add one or more `--prelude` files; they are injected in order before modules execute (define loaders, helpers, etc.).
- Rewrites (`--replace`) are applied in the order listed.
- `--vendor` inlines vendor modules from the listed roots, excluding anything matched by prior `--replace` and anything listed in `exclude=`.

#### --vendor spec syntax and name normalization

- Keys (comma-separated):
  - `path=<glob>` (repeatable)
  - `exclude=name:<module>` (repeatable)
  - `exclude=prefix:<prefix.>` (repeatable)
  - `suffix=<name>` (repeatable) — strip a trailing `.<name>` from the derived module name.

- Default behavior: if any `path` uses `?/init.lua`, luapack auto-detects the `init` suffix and normalizes names accordingly. This collapses duplicates like `mock_recoil.init` and `mock_recoil` into the canonical `mock_recoil`.

- Normalization order and consistency:
  1. Derive the module name from the matched file path and template.
  2. Apply suffix normalization (defaults + any explicit `suffix=` values).
  3. Apply `exclude=name:` and `exclude=prefix:` checks.
  4. Skip modules matched by `--replace` rules (including `match=path`).

- The same name normalization is used when:
  - Matching `--replace` rules with `match=exact|prefix`.
  - Recording module names discovered via `--path` for the first-party graph.

#### --replace flag syntax

- Common keys (comma-separated key=value):
  - `match`: `exact` | `prefix` | `path`
  - `old`: callee to match (e.g., `require`)
  - `new`: replacement callee (e.g., `bar_require`)
  - One of:
    - `name=<module>` (for `match=exact`)
    - `prefix=<prefix.>` (for `match=prefix`)
    - `path=<glob>` (for `match=path`, you may repeat `path=` within the same flag to provide multiple globs)
  - Optional: `arg={rest|full}`
    - Default `{rest}` for `match=prefix` (pass module without the matched prefix)
    - Default `{full}` for `match=exact` and `match=path` (pass full module name)

Match precedence: rules are evaluated in the order provided; first match wins. Prefer listing `exact` and `prefix` before any broad `path` rules.

`match=path` semantics:

- The module string inside `require("...")` is resolved to a file path using the configured `--path` templates (see “Module resolution with --path”).
- If the resolved file path matches any provided `path=<glob>`, the rule applies.
- If the module cannot be resolved to a file path, `match=path` rules do not apply to that call.

#### --vendor flag syntax

- Format: `--vendor='path=<glob>,exclude=name:<module>,exclude=prefix:<prefix.>'`
- Inlines any module resolvable under the given vendor roots, except:
  - modules already matched by any prior `--replace` rule
  - modules matched by `exclude=name:...` or `exclude=prefix:...`
- Multiple `--vendor` flags are allowed and processed in order; later flags see the effect of earlier ones.

### Transform rules

- Rewrite only calls of the form: `require("<literal>")` where `<literal>` matches a configured mapping.
- Preserve everything else (e.g., dynamic requires, local aliases, method calls):
  - Do not rewrite `local require = foo` or `obj.require("...")`.
  - Do not rewrite `require(prefix .. name)`.
- Always operate on the bundler's AST, not regex.

### Runtime loaders

Loader functions (e.g., `bar_require`, `evorts_require`) must be available before the bundle executes. Options:

- Provided by the game/mod runtime.
- Or injected at bundle prelude (with optional dev fallbacks):

```lua
bar_require = bar_require or function(name) return require("bar." .. name) end
evorts_require = evorts_require or function(name) return require("evorts." .. name) end
```

For games without `require`, adapt to their include mechanism (`VFS.Include`, etc.). Preludes should implement caching similar to `package.loaded` to avoid double loads.

### Runtime scoping model (bundle prelude)

luapack emits a small runtime that mirrors luabundle's scoping approach. The bundle defines an internal module table and a cached loader. User code is registered as functions and executed via a local `require` bound to the internal loader.

Header see [src/bundle.rs](src/bundle.rs):

```lua
-- luapack bundle v0.1.0 (auto-generated)
local __B_LOADED = {}
local __B_MODULES = {}

local function __B_REQUIRE(name)
  if __B_LOADED[name] ~= nil then
    return __B_LOADED[name] == true and nil or __B_LOADED[name]
  end
  local loader = __B_MODULES[name]
  if loader then
    local res = loader(__B_REQ_TO_PASS)
    __B_LOADED[name] = (res == nil) and true or res
    return res
  end
  error('module not found: ' .. name)
end

__B_REQ_TO_PASS = __B_REQUIRE
```

Module registration (for each source file bundled):

```lua
__B_MODULES['core.greet'] = function(require)
  -- file contents...
end
```

A special root module is registered with the entry file contents:

```lua
__B_MODULES['__root'] = function(require)
  -- entry file contents...
end
```

Preludes provided via repeated `--prelude` flags are appended after all modules are registered, so they can immediately `__B_REQUIRE("...")` or access globals. To execute a module at bundle end:

```lua
return __B_REQUIRE("__root")
```

or if the CLI specifies `--entry core.main`:

```lua
return __B_REQUIRE("core.main")
```

When `--replace` rules are present, luapack performs source-level rewrites of string-literal `require("...")` calls (AST-based) that match the configured rules. No runtime router is generated; the bundle continues to bind `require` to `__B_REQUIRE` inside bundled modules.

### Binding `require` inside bundled modules

- By default, modules are called with the router `__B_REQUIRE` passed as the `require` parameter.
- With `--bind-require global`, modules receive a wrapper that tries the host’s global require first and falls back to the bundle:

  ```lua
  -- pseudo
  if _G and _G.require then
    __B_REQ_TO_PASS = function(name)
      local ok, mod = pcall(_G.require, name)
      if ok then return mod end
      return __B_REQUIRE(name)
    end
  else
    __B_REQ_TO_PASS = __B_REQUIRE
  end
  ```

#### Why choose `--bind-require global`?

- Short answer: Use it when you want bundled code to call the host’s real `require` instead of the bundler’s internal loader—useful for runtime-provided modules, native/C libs, or when you need the host’s `package.searchers` behavior.

### IDE (lua-language-server) support

- Keep source imports as standard `require("...")` for full navigation.
- Settings:
  - `completion.requireSeparator = "."`
  - `runtime.path` includes:
    - `lua/?.lua`
    - `lua/?/init.lua`
    - `vendor/lua/?.lua`
    - `vendor/lua/?/init.lua`
- Optional EmmyLua stubs for loader functions (for tests or mixed code):

```lua
---@generic T
---@param name string
---@return T
function bar_require(name) end
```

### Error handling and caching

- Loaders should mirror `require` semantics:
  - Maintain a cache (like `package.loaded`).
  - Raise an error that references the original module name for easier debugging.

### Edge cases

- If a third-party package (e.g., `30log`) is provided by the engine as plain `require("30log")`, do not map it; let it pass through unchanged.
- If you want to vendor it under a namespace, add an explicit mapping for that name.

### Benefits

- Source remains portable and readable; bundle adapts to engine realities.
- Per-engine routing without polluting global module namespaces.
- Minimal friction for contributors and strong IDE experience.

### Example prelude

Skeleton:

```lua
local LOADED = {}
local function dotted_to_path(root, name) return root .. (name:gsub('%.','/')) .. '.lua' end
local function include(path, env) return VFS.Include(path, env or getfenv()) end

local function cached_load(key, path)
  if LOADED[key] ~= nil then return LOADED[key] end
  local mod = include(path)
  LOADED[key] = (mod == nil) and true or mod
  return mod
end

function bar_require(name)
  return cached_load('bar.'..name, dotted_to_path('LuaUI/bar/', name))
end

function evorts_require(name)
  return cached_load('evorts.'..name, dotted_to_path('LuaUI/evorts/', name))
end

-- Optional: used if you create a “prefer external or bundled” loader later
-- function prefer_external_or_bundle(name)
--   if require then local ok, mod = pcall(require, name); if ok then return mod end end
--   return __BUNDLED_REQUIRE(name)
-- end
```

### CLI examples

- Replace by prefix (strip prefix; default `{rest}`)

  ```bash
  luapack bundle lua/main.lua \
    --path "lua/?.lua" --path "lua/?/init.lua" \
    --replace "match=prefix,old=require,new=bar_require,prefix=bar." \
    --output dist/out.lua
  ```

  - Source: `require("bar.common.tablex")`
  - Output: `bar_require("common.tablex")`

- Replace exact (keep full; default `{full}`)

  ```bash
  luapack bundle lua/main.lua \
    --path "lua/?.lua" --path "lua/?/init.lua" \
    --replace "match=exact,old=require,new=require,name=30log" \
    --output dist/out.lua
  ```

  - Source: `require("30log")`
  - Output: unchanged (explicitly whitelisted to remain global)

- Replace by path (resolve via --path, then match glob)

  ```bash
  luapack bundle lua/main.lua \
    --path "lua/?.lua" --path "lua/?/init.lua" \
    --replace "match=path,old=require,new=evorts_require,path=vendor/evorts/**/init.lua" \
    --output dist/out.lua
  ```

  - If `require("evorts.core")` resolves to `vendor/evorts/core/init.lua`, it becomes:
    - `evorts_require("evorts.core")` (default `{full}`)

Notes

- Only string-literal `require("...")` calls are transformed; dynamic requires are preserved.
- For `match=path`, the module must resolve under `--path` first; globs apply to the resolved file path.

### Diagnostics and watch

- `--diagnostics` prints:
  - Rewrites performed (rule → module). For `match=path`, the resolved file path that matched the glob(s).
  - Vendor inclusions and excludes.
  - Residual `__B_REQUIRE()` not bundled or replaced.

### Module resolution with --path

- Resolution interprets dotted module names (`a.b.c`) under each `--path` template in order, using both `?.lua` and `?/init.lua` conventions.
- The first successful file found across all templates is used as the resolved file path.
- Note: `match=path` evaluates its glob(s) against this resolved, canonical file path. If resolution fails, `match=path` rules do not apply.
- `--watch` re-bundles on FS changes (paths, vendor roots, preludes), printing concise deltas.
