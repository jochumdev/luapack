# luapack

> [!NOTE]
> For now this is a quick prototype that works for me.

Lua bundler with replace/vendor modes.

## Install

Download the latest binaries from the GitHub Releases page.

## Quickstart

Bundle the simple example:

```bash
luapack bundle examples/simple/lua/main.lua \
  --lua "5.4" \
  --path "examples/simple/lua/?.lua" \
  --path "examples/simple/lua/?/init.lua" \
  --vendor "examples/simple/vendor/lua/?.lua" \
  --vendor "examples/simple/vendor/lua/?/init.lua" \
  --output examples/simple/dist/simple_bundle.lua
```

Run it with your Lua:

```bash
lua examples/simple/dist/simple_bundle.lua
```

## Configuration (optional)

Auto-discovery in current directory, supports: `luapack.yaml`, `luapack.yml` and
`luapack.json` as well.

Add a minimal `luapack.toml`:

```toml
[bundle]
lua = "5.4"
paths = [
  "lua/?.lua",
  "lua/?/init.lua",
]
vendors = [
  "vendor/lua/?.lua",
  "vendor/lua/?/init.lua",
]
# redact_base = "lua/"
output = "dist/simple_bundle.lua"
diagnostics = true
```

Usage:

```bash
# from the directory containing luapack.toml
luapack bundle lua/main.lua

# or pass it explicitly from anywhere
luapack bundle examples/simple/lua/main.lua --config examples/simple/luapack.toml
```

## Docs

See [`doc/bundler.md`](doc/bundler.md) for a deeper overview.

## Credits

- [full_moon](https://github.com/Kampfkarren/full-moon) used for lua AST parsing
- [luabundler](https://github.com/Benjamin-Dobell/luabundler) inspired the
  scoping approach
- [StyLua](https://github.com/JohnnyMorganz/StyLua) project structure,
  best-practices and full_moon integration

## Authors

- [@jochumdev](https://github.com/jochumdev) with AI assistant.

## License

MPL-2.0
