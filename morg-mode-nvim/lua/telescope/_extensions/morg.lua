--- Telescope extension for morg-mode
--- Provides pickers: todos, agenda, search, tags, headings

local has_telescope, telescope = pcall(require, "telescope")
if not has_telescope then
    return
end

local pickers = require("telescope.pickers")
local finders = require("telescope.finders")
local conf = require("telescope.config").values
local actions = require("telescope.actions")
local action_state = require("telescope.actions.state")
local entry_display = require("telescope.pickers.entry_display")
local morg = require("morg")

local M = {}

-- Helper: run morg command with --format json and return parsed table
local function morg_json(args, callback)
    local cmd_args = { "--format", "json" }
    vim.list_extend(cmd_args, args)

    morg.run(cmd_args, function(code, stdout, _)
        vim.schedule(function()
            if code ~= 0 or stdout == "" then
                callback({})
                return
            end
            local ok, data = pcall(vim.json.decode, stdout)
            if ok and type(data) == "table" then
                callback(data)
            else
                callback({})
            end
        end)
    end)
end

-- Helper: open file at line
local function goto_entry(entry)
    if entry.file and entry.line then
        vim.cmd("edit " .. entry.file)
        vim.api.nvim_win_set_cursor(0, { entry.line, 0 })
    end
end

--- Telescope picker: TODOs
function M.todos(opts)
    opts = opts or {}
    morg_json({ "todos", morg.root() }, function(items)
        if #items == 0 then
            vim.notify("No TODOs found", vim.log.levels.INFO)
            return
        end

        local displayer = entry_display.create({
            separator = " ",
            items = {
                { width = 6 }, -- status
                { width = 3 }, -- priority
                { width = 6 }, -- effort
                { remaining = true }, -- text
            },
        })

        pickers
            .new(opts, {
                prompt_title = "morg todos",
                finder = finders.new_table({
                    results = items,
                    entry_maker = function(item)
                        return {
                            value = item,
                            display = function(entry)
                                return displayer({
                                    {
                                        entry.value.status,
                                        entry.value.status == "DONE" and "TelescopeResultsComment" or "WarningMsg",
                                    },
                                    { entry.value.priority or "", "DiagnosticError" },
                                    { entry.value.effort or "", "Comment" },
                                    entry.value.text
                                        .. (entry.value.heading and (" (" .. entry.value.heading .. ")") or ""),
                                })
                            end,
                            ordinal = item.status .. " " .. item.text .. " " .. (item.heading or ""),
                            file = item.file,
                            line = item.line,
                        }
                    end,
                }),
                sorter = conf.generic_sorter(opts),
                attach_mappings = function(prompt_bufnr, _)
                    actions.select_default:replace(function()
                        actions.close(prompt_bufnr)
                        local entry = action_state.get_selected_entry()
                        goto_entry(entry)
                    end)
                    return true
                end,
            })
            :find()
    end)
end

--- Telescope picker: Agenda
function M.agenda(opts)
    opts = opts or {}
    morg_json({ "agenda", morg.root() }, function(items)
        if #items == 0 then
            vim.notify("No agenda entries", vim.log.levels.INFO)
            return
        end

        local displayer = entry_display.create({
            separator = " ",
            items = {
                { width = 10 }, -- date
                { width = 10 }, -- kind
                { remaining = true }, -- description
            },
        })

        pickers
            .new(opts, {
                prompt_title = "morg agenda",
                finder = finders.new_table({
                    results = items,
                    entry_maker = function(item)
                        return {
                            value = item,
                            display = function(entry)
                                local kind_hl = "Comment"
                                if entry.value.kind == "DEADLINE" then
                                    kind_hl = "DiagnosticError"
                                elseif entry.value.kind == "SCHEDULED" then
                                    kind_hl = "DiagnosticInfo"
                                elseif entry.value.kind == "EVENT" then
                                    kind_hl = "DiagnosticHint"
                                end

                                return displayer({
                                    { entry.value.date, "Number" },
                                    { entry.value.kind, kind_hl },
                                    entry.value.description or "",
                                })
                            end,
                            ordinal = item.date .. " " .. item.kind .. " " .. (item.description or ""),
                            file = item.file,
                            line = item.line,
                        }
                    end,
                }),
                sorter = conf.generic_sorter(opts),
                attach_mappings = function(prompt_bufnr, _)
                    actions.select_default:replace(function()
                        actions.close(prompt_bufnr)
                        goto_entry(action_state.get_selected_entry())
                    end)
                    return true
                end,
            })
            :find()
    end)
end

--- Telescope picker: Search (live query)
function M.search(opts)
    opts = opts or {}

    -- Start with a prompt for the query
    vim.ui.input({ prompt = "morg search: " }, function(query)
        if not query or query == "" then
            return
        end

        morg_json({ "search", query, morg.root() }, function(items)
            if #items == 0 then
                vim.notify('No matches for "' .. query .. '"', vim.log.levels.INFO)
                return
            end

            local displayer = entry_display.create({
                separator = " ",
                items = {
                    { width = 8 }, -- kind
                    { remaining = true }, -- text
                },
            })

            pickers
                .new(opts, {
                    prompt_title = "morg search: " .. query,
                    finder = finders.new_table({
                        results = items,
                        entry_maker = function(item)
                            return {
                                value = item,
                                display = function(entry)
                                    return displayer({
                                        { entry.value.kind, "Comment" },
                                        entry.value.text,
                                    })
                                end,
                                ordinal = item.kind .. " " .. item.text .. " " .. (item.heading or ""),
                                file = item.file,
                                line = item.line,
                            }
                        end,
                    }),
                    sorter = conf.generic_sorter(opts),
                    attach_mappings = function(prompt_bufnr, _)
                        actions.select_default:replace(function()
                            actions.close(prompt_bufnr)
                            goto_entry(action_state.get_selected_entry())
                        end)
                        return true
                    end,
                })
                :find()
        end)
    end)
end

--- Telescope picker: Tags (all unique tags across files)
function M.tags(opts)
    opts = opts or {}
    -- Use search with tags-only to get all tags
    morg_json({ "search", "#", morg.root(), "--tags-only" }, function(items)
        -- Deduplicate by tag text
        local seen = {}
        local unique = {}
        for _, item in ipairs(items) do
            if not seen[item.text] then
                seen[item.text] = true
                table.insert(unique, item)
            end
        end

        if #unique == 0 then
            vim.notify("No tags found", vim.log.levels.INFO)
            return
        end

        pickers
            .new(opts, {
                prompt_title = "morg tags",
                finder = finders.new_table({
                    results = unique,
                    entry_maker = function(item)
                        return {
                            value = item,
                            display = item.text,
                            ordinal = item.text,
                            file = item.file,
                            line = item.line,
                        }
                    end,
                }),
                sorter = conf.generic_sorter(opts),
                attach_mappings = function(prompt_bufnr, _)
                    actions.select_default:replace(function()
                        actions.close(prompt_bufnr)
                        goto_entry(action_state.get_selected_entry())
                    end)
                    return true
                end,
            })
            :find()
    end)
end

--- Telescope picker: Headings
function M.headings(opts)
    opts = opts or {}
    morg_json({ "search", "#", morg.root(), "--headings-only" }, function(items)
        if #items == 0 then
            vim.notify("No headings found", vim.log.levels.INFO)
            return
        end

        pickers
            .new(opts, {
                prompt_title = "morg headings",
                finder = finders.new_table({
                    results = items,
                    entry_maker = function(item)
                        return {
                            value = item,
                            display = item.text,
                            ordinal = item.text,
                            file = item.file,
                            line = item.line,
                        }
                    end,
                }),
                sorter = conf.generic_sorter(opts),
                attach_mappings = function(prompt_bufnr, _)
                    actions.select_default:replace(function()
                        actions.close(prompt_bufnr)
                        goto_entry(action_state.get_selected_entry())
                    end)
                    return true
                end,
            })
            :find()
    end)
end

return telescope.register_extension({
    exports = {
        todos = M.todos,
        agenda = M.agenda,
        search = M.search,
        tags = M.tags,
        headings = M.headings,
    },
})
