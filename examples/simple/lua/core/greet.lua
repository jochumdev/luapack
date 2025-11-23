local M = {}

function M.hello(name)
	return ("Hello, %s"):format(name or "world")
end

return M
