-- luapack bundle v0.1.1 auto-generated: DO NOT EDIT
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

__B_REQ_TO_PASS = __B_REQUIRE

-- module: core.greet  (from core/greet.lua)
__B_MODULES['core.greet'] = function(require)
local M = {}

function M.hello(name)
	return ("Hello, %s"):format(name or "world")
end

return M
end

-- module: core.greet_setup  (from core/greet_setup.lua)
__B_MODULES['core.greet_setup'] = function(require)
local M = {
	greeting = "Hello",
}

function M.setup(greeting)
	M.greeting = greeting or M.greeting
end

function M.hello(name)
	return ("%s, %s"):format(M.greeting, name or "world")
end

return M
end

-- vendor module: 30log
__B_MODULES['30log'] = function(require)
local class = {}
class._VERSION = "30log mock"

return class
end

-- vendor module: bar.common.tablex
__B_MODULES['bar.common.tablex'] = function(require)
local M = {}

function M.sum(t)
  local s = 0
  for _, v in ipairs(t) do s = s + v end
  return s
end

return M
end

-- vendor module: mock_recoil
__B_MODULES['mock_recoil'] = function(require)
Spring = {}
function Spring.Echo(arg, ...)
	print(arg, ...)
end
end

-- root module: __root
__B_MODULES['__root'] = function(require)
require("mock_recoil")

local class = require("30log")
local greet = require("core.greet")

require("core.greet_setup").setup("Salve")
Spring.Echo("[main] " .. require("core.greet_setup").hello())

Spring.Echo("[main] " .. greet.hello("luapack"))
Spring.Echo("[main] 30log version: " .. tostring(class and class._VERSION))
end

return __B_REQUIRE('__root')
