-- Add vendor paths so global require can find bar.* during testing
package.path = table.concat({
  "examples/replace/vendor/lua/?.lua",
  "examples/replace/vendor/lua/?/init.lua",
  package.path,
}, ";")

-- Define vendor loader used by --replace rules; route to runtime require
bar_require = bar_require or function(name)
  return require('bar.' .. name)
end
