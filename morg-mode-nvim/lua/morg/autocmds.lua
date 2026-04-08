--- Autocommands for morg-mode
local M = {}

function M.setup()
    local group = vim.api.nvim_create_augroup("MorgMode", { clear = true })
    local config = require("morg").config

    -- Set up buffer-local keymaps and options for morg files
    vim.api.nvim_create_autocmd("FileType", {
        group = group,
        pattern = { "markdown", "morg" },
        callback = function()
            require("morg.keymaps").setup_buffer()
        end,
    })

    -- Auto-tangle on save
    if config.auto_tangle then
        vim.api.nvim_create_autocmd("BufWritePost", {
            group = group,
            pattern = { "*.md", "*.morg", "*.markdown" },
            callback = function()
                require("morg.commands.tangle").run()
            end,
        })
    end

    -- Auto-lint on save
    if config.auto_lint then
        vim.api.nvim_create_autocmd("BufWritePost", {
            group = group,
            pattern = { "*.md", "*.morg", "*.markdown" },
            callback = function()
                require("morg.commands.lint").run()
            end,
        })
    end
end

return M
