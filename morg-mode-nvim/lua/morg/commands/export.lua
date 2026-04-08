local M = {}
local morg = require("morg")

function M.run(output_path)
    local file = vim.fn.expand("%:p")
    local out = output_path or (vim.fn.expand("%:p:r") .. ".html")

    morg.run({ "export", file, "-o", out }, function(code, _, stderr)
        vim.schedule(function()
            if code ~= 0 then
                vim.notify("morg export failed: " .. stderr, vim.log.levels.ERROR)
                return
            end
            vim.notify("Exported to " .. out, vim.log.levels.INFO)

            -- Try to open in browser
            local open_cmd = nil
            if vim.fn.has("mac") == 1 then
                open_cmd = "open"
            elseif vim.fn.has("unix") == 1 then
                open_cmd = "xdg-open"
            end
            if open_cmd then
                vim.fn.jobstart({ open_cmd, out }, { detach = true })
            end
        end)
    end)
end

return M
