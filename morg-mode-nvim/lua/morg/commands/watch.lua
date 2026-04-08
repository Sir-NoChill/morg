local M = {}
local morg = require("morg")

M._job_id = nil

function M.run(subcmd)
    if M._job_id then
        vim.fn.jobstop(M._job_id)
        M._job_id = nil
        vim.notify("morg watch stopped", vim.log.levels.INFO)
        return
    end

    subcmd = subcmd or "tangle"
    local cmd = { morg.binary(), "watch", morg.root(), "--command", subcmd }

    M._job_id = vim.fn.jobstart(cmd, {
        on_stderr = function(_, data)
            if data then
                for _, line in ipairs(data) do
                    if line ~= "" then
                        vim.schedule(function()
                            vim.notify("morg watch: " .. line, vim.log.levels.INFO)
                        end)
                    end
                end
            end
        end,
        on_exit = function()
            M._job_id = nil
            vim.schedule(function()
                vim.notify("morg watch exited", vim.log.levels.INFO)
            end)
        end,
    })

    vim.notify("morg watch started (" .. subcmd .. ")", vim.log.levels.INFO)
end

return M
