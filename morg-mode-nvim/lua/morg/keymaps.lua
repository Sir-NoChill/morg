--- Keybindings for morg-mode
local M = {}

function M.setup(prefix)
    local map = function(suffix, cmd, desc)
        vim.keymap.set("n", prefix .. suffix, cmd, { desc = "morg: " .. desc })
    end

    -- Workflow commands
    map("t", "<cmd>MorgTodos<cr>", "Todos (quickfix)")
    map("a", "<cmd>MorgAgenda<cr>", "Agenda")
    map("T", "<cmd>MorgTangle<cr>", "Tangle")
    map("l", "<cmd>MorgLint<cr>", "Lint")
    map("e", "<cmd>MorgExport<cr>", "Export HTML")
    map("s", "<cmd>MorgSearch<cr>", "Search")
    map("r", "<cmd>MorgTime<cr>", "Time Report")
    map("c", "<cmd>MorgColumns<cr>", "Column View")
    map("R", "<cmd>MorgRefs<cr>", "Validate Refs")
    map("w", "<cmd>MorgWatch<cr>", "Toggle Watch")

    -- Telescope pickers (if telescope is available)
    local has_telescope = pcall(require, "telescope")
    if has_telescope then
        map("ft", "<cmd>Telescope morg todos<cr>", "Telescope: Todos")
        map("fa", "<cmd>Telescope morg agenda<cr>", "Telescope: Agenda")
        map("fs", "<cmd>Telescope morg search<cr>", "Telescope: Search")
        map("fg", "<cmd>Telescope morg tags<cr>", "Telescope: Tags")
        map("fh", "<cmd>Telescope morg headings<cr>", "Telescope: Headings")
    end

    -- Editing helpers (buffer-local, set in ftplugin)
    -- These are registered per-buffer in autocmds.lua
end

--- Buffer-local keymaps for morg files
function M.setup_buffer()
    local buf_map = function(mode, lhs, rhs, desc)
        vim.keymap.set(mode, lhs, rhs, { buffer = true, desc = "morg: " .. desc })
    end

    -- Checkbox toggle
    buf_map("n", "<C-space>", function()
        require("morg.editing").toggle_checkbox()
    end, "Toggle checkbox")

    -- Insert tags
    buf_map("n", "<leader>it", function()
        require("morg.editing").insert_tag("todo")
    end, "Insert #todo")
    buf_map("n", "<leader>id", function()
        require("morg.editing").insert_tag("done")
    end, "Insert #done")
    buf_map("n", "<leader>iD", function()
        require("morg.editing").insert_date_tag("deadline")
    end, "Insert #deadline")
    buf_map("n", "<leader>iS", function()
        require("morg.editing").insert_date_tag("scheduled")
    end, "Insert #scheduled")
    buf_map("n", "<leader>ic", function()
        require("morg.editing").insert_clock_in()
    end, "Clock in")
    buf_map("n", "<leader>iC", function()
        require("morg.editing").insert_clock_out()
    end, "Clock out")
    buf_map("n", "<leader>ip", function()
        require("morg.editing").insert_properties()
    end, "Insert properties block")

    -- Heading promotion/demotion
    buf_map("n", ">>", function()
        require("morg.editing").demote_heading()
    end, "Demote heading")
    buf_map("n", "<<", function()
        require("morg.editing").promote_heading()
    end, "Promote heading")
end

return M
