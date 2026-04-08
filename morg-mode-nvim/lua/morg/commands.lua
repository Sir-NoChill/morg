--- Neovim command registrations for morg-mode
local M = {}
local morg = require("morg")

function M.setup()
    local cmd = vim.api.nvim_create_user_command

    -- :MorgTodos — show todos in quickfix
    cmd("MorgTodos", function()
        require("morg.commands.todos").run()
    end, { desc = "morg: List TODOs in quickfix" })

    -- :MorgAgenda — show agenda in a split
    cmd("MorgAgenda", function()
        require("morg.commands.agenda").run()
    end, { desc = "morg: Show agenda" })

    -- :MorgTangle — tangle current file
    cmd("MorgTangle", function()
        require("morg.commands.tangle").run()
    end, { desc = "morg: Tangle current file" })

    -- :MorgLint — lint current file, populate diagnostics
    cmd("MorgLint", function()
        require("morg.commands.lint").run()
    end, { desc = "morg: Lint current file" })

    -- :MorgExport — export current file to HTML and open
    cmd("MorgExport", function(opts)
        require("morg.commands.export").run(opts.args ~= "" and opts.args or nil)
    end, { nargs = "?", desc = "morg: Export to HTML", complete = "file" })

    -- :MorgSearch <query> — search across project
    cmd("MorgSearch", function(opts)
        require("morg.commands.search").run(opts.args)
    end, { nargs = 1, desc = "morg: Search across files" })

    -- :MorgTime — show time report in a split
    cmd("MorgTime", function()
        require("morg.commands.time").run()
    end, { desc = "morg: Time tracking report" })

    -- :MorgColumns — show column view in a split
    cmd("MorgColumns", function()
        require("morg.commands.columns").run()
    end, { desc = "morg: Column view" })

    -- :MorgCapture <template> <input> — quick capture
    cmd("MorgCapture", function(opts)
        local parts = vim.split(opts.args, " ", { trimempty = true })
        if #parts < 2 then
            vim.notify("Usage: :MorgCapture <template> <input>", vim.log.levels.WARN)
            return
        end
        local template = table.remove(parts, 1)
        local input = table.concat(parts, " ")
        require("morg.commands.capture").run(template, input)
    end, { nargs = "+", desc = "morg: Capture entry" })

    -- :MorgRefs — validate cross-file references
    cmd("MorgRefs", function()
        require("morg.commands.refs").run()
    end, { desc = "morg: Validate references" })

    -- :MorgArchive — archive #archive subtrees
    cmd("MorgArchive", function(opts)
        local dry_run = opts.bang
        require("morg.commands.archive").run(dry_run)
    end, { bang = true, desc = "morg: Archive subtrees (! for dry run)" })

    -- :MorgId — assign UUIDs to headings
    cmd("MorgId", function(opts)
        local dry_run = opts.bang
        require("morg.commands.id").run(dry_run)
    end, { bang = true, desc = "morg: Assign heading IDs (! for dry run)" })

    -- :MorgRefile <target> — refile heading at cursor to target
    cmd("MorgRefile", function(opts)
        require("morg.commands.refile").run(opts.args)
    end, { nargs = 1, desc = "morg: Refile heading at cursor", complete = "file" })

    -- :MorgWatch — start watcher
    cmd("MorgWatch", function(opts)
        local subcmd = opts.args ~= "" and opts.args or "tangle"
        require("morg.commands.watch").run(subcmd)
    end, { nargs = "?", desc = "morg: Start file watcher" })
end

return M
