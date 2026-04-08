local M = {}
local morg = require("morg")

function M.run(query)
    if not query or query == "" then
        vim.ui.input({ prompt = "morg search: " }, function(input)
            if input and input ~= "" then
                M.run(input)
            end
        end)
        return
    end

    morg.run({ "search", query, morg.root() }, function(code, stdout, _)
        vim.schedule(function()
            if code ~= 0 then
                vim.notify("morg search failed", vim.log.levels.ERROR)
                return
            end

            local items = {}
            for line in stdout:gmatch("[^\n]+") do
                local file, lnum = line:match("^([^:]+):(%d+)")
                if file and lnum then
                    local text = line:match(":%d+%s+(.+)$") or line
                    table.insert(items, {
                        filename = file,
                        lnum = tonumber(lnum),
                        text = text,
                    })
                end
            end

            if #items == 0 then
                vim.notify('No matches for "' .. query .. '"', vim.log.levels.INFO)
                return
            end

            vim.fn.setqflist(items, "r")
            vim.fn.setqflist({}, "a", { title = "morg search: " .. query })
            vim.cmd("copen")
        end)
    end)
end

return M
