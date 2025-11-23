require("mock_recoil")

local class = require("30log")
local greet = require("core.greet")

require("core.greet_setup").setup("Salve")
Spring.Echo("[main] " .. require("core.greet_setup").hello())

Spring.Echo("[main] " .. greet.hello("luapack"))
Spring.Echo("[main] 30log version: " .. tostring(class and class._VERSION))
