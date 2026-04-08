local M = {}
local morg = require("morg")

function M.run()
    morg.run({ "time", morg.root() }, function(code, stdout, _)
        vim.schedule(function()
            if code ~= 0 then
                vim.notify("morg time failed", vim.log.levels.ERROR)
                return
            end
            vim.cmd("botright new")
            local buf = vim.api.nvim_get_current_buf()
            vim.bo[buf].buftype = "nofile"
            vim.bo[buf].bufhidden = "wipe"
            vim.bo[buf].filetype = "morg-time"
            vim.bo[buf].swapfile = false
            vim.api.nvim_buf_set_name(buf, "morg://time")
            vim.api.nvim_buf_set_lines(buf, 0, -1, false, vim.split(stdout, "\n", { trimempty = true }))
            vim.bo[buf].modifiable = false
            vim.keymap.set("n", "q", "<cmd>bdelete<cr>", { buffer = buf })
        end)
    end)
end

return M
