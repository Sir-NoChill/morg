local M = {}
local morg = require("morg")

function M.run()
    local file = vim.fn.expand("%:p")
    morg.run({ "tangle", file }, function(code, stdout, stderr)
        vim.schedule(function()
            if code ~= 0 then
                vim.notify("morg tangle failed: " .. stderr, vim.log.levels.ERROR)
            else
                vim.notify(vim.trim(stdout), vim.log.levels.INFO)
            end
        end)
    end)
end

return M
