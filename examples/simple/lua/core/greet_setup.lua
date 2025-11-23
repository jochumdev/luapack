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
