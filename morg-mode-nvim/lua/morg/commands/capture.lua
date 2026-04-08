local M = {}
local morg = require("morg")

function M.run(template, input)
    morg.run({ "capture", template, input }, function(code, stdout, stderr)
        vim.schedule(function()
            if code ~= 0 then
                vim.notify("morg capture failed: " .. stderr, vim.log.levels.ERROR)
            else
                vim.notify(vim.trim(stdout), vim.log.levels.INFO)
            end
        end)
    end)
end

return M
