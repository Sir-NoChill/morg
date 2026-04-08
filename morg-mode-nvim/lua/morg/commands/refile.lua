local M = {}
local morg = require("morg")

function M.run(target)
    local file = vim.fn.expand("%:p")
    local lnum = vim.api.nvim_win_get_cursor(0)[1]
    local source = file .. ":" .. lnum

    morg.run({ "refile", source, target }, function(code, stdout, stderr)
        vim.schedule(function()
            if code ~= 0 then
                vim.notify("morg refile failed: " .. stderr, vim.log.levels.ERROR)
                return
            end
            vim.notify(vim.trim(stdout), vim.log.levels.INFO)
            vim.cmd("edit!")
        end)
    end)
end

return M
