local M = {}
local morg = require("morg")

function M.run(dry_run)
    local file = vim.fn.expand("%:p")
    local args = { "archive", file }
    if dry_run then
        table.insert(args, "--dry-run")
    end
    morg.run(args, function(code, stdout, stderr)
        vim.schedule(function()
            if code ~= 0 then
                vim.notify("morg archive failed: " .. stderr, vim.log.levels.ERROR)
                return
            end
            vim.notify(vim.trim(stdout), vim.log.levels.INFO)
            if not dry_run then
                vim.cmd("edit!") -- reload file
            end
        end)
    end)
end

return M
