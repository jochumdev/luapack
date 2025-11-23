local M = {}

function M.sum(t)
  local s = 0
  for _, v in ipairs(t) do s = s + v end
  return s
end

return M
