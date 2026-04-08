local M = {}
local morg = require("morg")

function M.run()
    morg.run({ "agenda", morg.root() }, function(code, stdout, _)
        vim.schedule(function()
            if code ~= 0 then
                vim.notify("morg agenda failed", vim.log.levels.ERROR)
                return
            end

            -- Show in a scratch split
            vim.cmd("botright new")
            local buf = vim.api.nvim_get_current_buf()
            vim.bo[buf].buftype = "nofile"
            vim.bo[buf].bufhidden = "wipe"
            vim.bo[buf].filetype = "morg-agenda"
            vim.bo[buf].swapfile = false
            vim.api.nvim_buf_set_name(buf, "morg://agenda")

            local lines = vim.split(stdout, "\n", { trimempty = true })
            vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
            vim.bo[buf].modifiable = false

            -- gd to go to source location
            vim.keymap.set("n", "gd", function()
                local line = vim.api.nvim_get_current_line()
                local file, lnum = line:match("%-%-  ([^:]+):(%d+)$")
                if file and lnum then
                    vim.cmd("wincmd p")
                    vim.cmd("edit " .. file)
                    vim.api.nvim_win_set_cursor(0, { tonumber(lnum), 0 })
                end
            end, { buffer = buf, desc = "Go to source" })

            vim.keymap.set("n", "q", "<cmd>bdelete<cr>", { buffer = buf, desc = "Close" })
        end)
    end)
end

return M
