local M = {}
local morg = require("morg")

function M.run()
    morg.run({ "todos", morg.root() }, function(code, stdout, _)
        vim.schedule(function()
            if code ~= 0 then
                vim.notify("morg todos failed", vim.log.levels.ERROR)
                return
            end

            local items = {}
            for line in stdout:gmatch("[^\n]+") do
                -- Parse: [STATUS] text  -- file:line
                local file, lnum = line:match("%-%-  ([^:]+):(%d+)$")
                local status = line:match("^%[(%w+)%]")
                local text = line:match("^%[%w+%]%s*(.-)%s+%-%-")
                if file and lnum and text then
                    table.insert(items, {
                        filename = file,
                        lnum = tonumber(lnum),
                        text = string.format("[%s] %s", status or "?", text),
                        type = (status == "DONE") and "I" or "W",
                    })
                end
            end

            if #items == 0 then
                vim.notify("No TODOs found.", vim.log.levels.INFO)
                return
            end

            vim.fn.setqflist(items, "r")
            vim.fn.setqflist({}, "a", { title = "morg todos" })
            vim.cmd("copen")
        end)
    end)
end

return M
