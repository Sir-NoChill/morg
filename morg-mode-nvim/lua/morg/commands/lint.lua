local M = {}
local morg = require("morg")

local ns = vim.api.nvim_create_namespace("morg_lint")

function M.run()
    local file = vim.fn.expand("%:p")
    local bufnr = vim.api.nvim_get_current_buf()

    morg.run({ "lint", file }, function(code, stdout, _)
        vim.schedule(function()
            vim.diagnostic.reset(ns, bufnr)

            local diagnostics = {}
            for line in stdout:gmatch("[^\n]+") do
                -- Parse: file:line  [severity] message
                local lnum, severity, msg = line:match(":(%d+)%s+%[(%w+)%]%s+(.+)$")
                if lnum and severity and msg then
                    local sev = vim.diagnostic.severity.WARN
                    if severity == "error" then
                        sev = vim.diagnostic.severity.ERROR
                    end
                    table.insert(diagnostics, {
                        lnum = tonumber(lnum) - 1,
                        col = 0,
                        message = msg,
                        severity = sev,
                        source = "morg",
                    })
                end
            end

            vim.diagnostic.set(ns, bufnr, diagnostics)

            if #diagnostics == 0 then
                vim.notify("morg lint: no issues", vim.log.levels.INFO)
            else
                vim.notify(string.format("morg lint: %d issue(s)", #diagnostics), vim.log.levels.WARN)
            end
        end)
    end)
end

return M
