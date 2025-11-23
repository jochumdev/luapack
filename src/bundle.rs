use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::ValueEnum;
use handlebars::Handlebars;

use crate::graph::ModuleGraph;
use crate::options::NameNormalizer;
use crate::replace::ReplaceRule;
use crate::resolve::ModuleResolver;
use crate::transform::transform_requires;

#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum BindRequire {
    Router,
    Global,
}

pub struct BundleCtx<'a> {
    pub preludes: &'a [PathBuf],
    pub entry: Option<&'a str>,
    pub replaces: &'a [ReplaceRule],
    pub vendor_mods: &'a std::collections::HashMap<String, PathBuf>,
    pub entry_source: &'a str,
    pub entry_path: &'a Path,
    pub bind: BindRequire,
    pub resolver: Option<&'a ModuleResolver>,
    pub redact_base: Option<PathBuf>,
    pub normalizer: &'a NameNormalizer,
}

pub fn lua_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "\\'"))
}

#[derive(serde::Serialize)]
struct HeaderCtx {
    global: bool,
    version: String,
}

fn render_header(bind: BindRequire) -> String {
    // Handlebars template for the bundle header. Switches behavior based on `global`.
    let tpl = r#"-- luapack bundle v{{version}} auto-generated: DO NOT EDIT
local __B_LOADED = {}
local __B_MODULES = {}
local __B_REQ_TO_PASS

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

{{#if global}}
__B_REQ_TO_PASS = (function()
  if require then
    return function(name)
      local ok, mod = pcall(require, name)
      if ok then return mod end
      return __B_REQUIRE(name)
    end
  else
    return __B_REQUIRE
  end
end)()
{{else}}
__B_REQ_TO_PASS = __B_REQUIRE
{{/if}}

"#;
    let mut hbs = Handlebars::new();
    let _ = hbs.register_template_string("header", tpl);
    let ctx = HeaderCtx {
        global: matches!(bind, BindRequire::Global),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    hbs.render("header", &ctx)
        .unwrap_or_else(|_| tpl.to_string())
}

pub fn generate_bundle(graph: &ModuleGraph, ctx: BundleCtx) -> Result<(String, usize)> {
    let mut out = String::new();
    let mut total_rewrites = 0usize;

    let header = render_header(ctx.bind);
    out.push_str(&header);

    // Base directory for redacting absolute paths
    let base = ctx
        .redact_base
        .clone()
        .or_else(|| std::env::current_dir().ok());

    let mut mods: Vec<_> = graph.first_party.iter().collect();
    mods.sort_by(|a, b| a.0.cmp(b.0));
    let mut emitted: HashSet<String> = HashSet::new();
    for (name, path) in mods {
        emitted.insert(name.clone());
        let rel = base.as_ref().and_then(|c| path.strip_prefix(c).ok());
        match rel {
            Some(rp) => out.push_str(&format!("-- module: {}  (from {})\n", name, rp.display())),
            None => out.push_str(&format!("-- module: {}\n", name)),
        }
        out.push_str(&format!(
            "__B_MODULES[{}] = function(require)\n",
            lua_quote(name)
        ));
        if let Ok(code) = fs::read_to_string(path) {
            let (code, c) = if !ctx.replaces.is_empty() {
                transform_requires(
                    &code,
                    ctx.replaces,
                    Some(path.as_path()),
                    ctx.resolver,
                    ctx.normalizer,
                )
            } else {
                (code, 0)
            };
            total_rewrites += c;
            out.push_str(&code);
            if !code.ends_with('\n') {
                out.push('\n');
            }
        }
        out.push_str("end\n\n");
    }

    let mut vmods: Vec<_> = ctx.vendor_mods.iter().collect();
    vmods.sort_by(|a, b| a.0.cmp(b.0));
    for (name, path) in vmods {
        if emitted.contains(name) {
            continue;
        }
        let rel = base.as_ref().and_then(|c| path.strip_prefix(c).ok());
        match rel {
            Some(rp) => out.push_str(&format!(
                "-- vendor module: {}  (from {})\n",
                name,
                rp.display()
            )),
            None => out.push_str(&format!("-- vendor module: {}\n", name)),
        }
        out.push_str(&format!(
            "__B_MODULES[{}] = function(require)\n",
            lua_quote(name)
        ));
        if let Ok(code) = fs::read_to_string(path) {
            out.push_str(&code);
            if !code.ends_with('\n') {
                out.push('\n');
            }
        }
        out.push_str("end\n\n");
    }

    out.push_str("-- root module: __root\n");
    out.push_str("__B_MODULES['__root'] = function(require)\n");
    let (entry_src, entry_c) = if !ctx.replaces.is_empty() {
        transform_requires(
            ctx.entry_source,
            ctx.replaces,
            Some(ctx.entry_path),
            ctx.resolver,
            ctx.normalizer,
        )
    } else {
        (ctx.entry_source.to_string(), 0)
    };
    total_rewrites += entry_c;
    out.push_str(&entry_src);
    if !ctx.entry_source.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("end\n\n");

    for p in ctx.preludes {
        if let Ok(txt) = fs::read_to_string(p) {
            let rel = base.as_ref().and_then(|c| p.strip_prefix(c).ok());
            match rel {
                Some(rp) => {
                    out.push_str("-- prelude: ");
                    out.push_str(&rp.display().to_string());
                    out.push('\n');
                }
                None => {
                    out.push_str("-- prelude\n");
                }
            }
            out.push_str(&txt);
            if !txt.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
        }
    }

    match ctx.entry {
        Some(entry_mod) => out.push_str(&format!("return __B_REQUIRE({})\n", lua_quote(entry_mod))),
        None => out.push_str("return __B_REQUIRE('__root')\n"),
    }

    Ok((out, total_rewrites))
}
