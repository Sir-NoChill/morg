--- Editing helpers for morg files
local M = {}

--- Toggle checkbox on current line: [ ] <-> [x], or insert [ ] if none
function M.toggle_checkbox()
    local line = vim.api.nvim_get_current_line()

    if line:match("%[x%]") or line:match("%[X%]") then
        local new = line:gsub("%[x%]", "[ ]"):gsub("%[X%]", "[ ]")
        vim.api.nvim_set_current_line(new)
    elseif line:match("%[ %]") then
        local new = line:gsub("%[ %]", "[x]")
        vim.api.nvim_set_current_line(new)
    elseif line:match("^%s*[-*+]%s") then
        -- List item without checkbox — add one
        local new = line:gsub("^(%s*[-*+])(%s)", "%1 [ ]%2")
        vim.api.nvim_set_current_line(new)
    end
end

--- Insert a tag at cursor position
--- @param tag string tag name (without #)
function M.insert_tag(tag)
    local row, col = unpack(vim.api.nvim_win_get_cursor(0))
    local line = vim.api.nvim_get_current_line()
    local before = line:sub(1, col)
    local after = line:sub(col + 1)

    -- Add space before tag if needed
    local prefix = ""
    if before ~= "" and not before:match("%s$") then
        prefix = " "
    end

    vim.api.nvim_set_current_line(before .. prefix .. "#" .. tag .. " " .. after)
    vim.api.nvim_win_set_cursor(0, { row, col + #prefix + #tag + 2 })
end

--- Insert a date-bearing tag with today's date
--- @param tag string tag name (deadline, scheduled, date, event)
function M.insert_date_tag(tag)
    local date = os.date("%Y-%m-%d")
    local row, col = unpack(vim.api.nvim_win_get_cursor(0))
    local line = vim.api.nvim_get_current_line()

    -- If line is empty or we're at end, insert as block-level tag
    if line:match("^%s*$") then
        vim.api.nvim_set_current_line("#" .. tag .. " " .. date)
        vim.api.nvim_win_set_cursor(0, { row, #tag + 1 + #date + 1 })
    else
        -- Insert inline
        local before = line:sub(1, col)
        local after = line:sub(col + 1)
        local prefix = (before ~= "" and not before:match("%s$")) and " " or ""
        vim.api.nvim_set_current_line(before .. prefix .. "#" .. tag .. " " .. date .. after)
    end
end

--- Insert #clock-in with current datetime
function M.insert_clock_in()
    local dt = os.date("%Y-%m-%dT%H:%M")
    local row = vim.api.nvim_win_get_cursor(0)[1]
    vim.api.nvim_buf_set_lines(0, row, row, false, { "#clock-in " .. dt })
    vim.api.nvim_win_set_cursor(0, { row + 1, 0 })
end

--- Insert #clock-out with current datetime
function M.insert_clock_out()
    local dt = os.date("%Y-%m-%dT%H:%M")
    local row = vim.api.nvim_win_get_cursor(0)[1]
    vim.api.nvim_buf_set_lines(0, row, row, false, { "#clock-out " .. dt })
    vim.api.nvim_win_set_cursor(0, { row + 1, 0 })
end

--- Insert a #properties / #end block after the current heading
function M.insert_properties()
    local row = vim.api.nvim_win_get_cursor(0)[1]
    local line = vim.api.nvim_get_current_line()

    -- Find if we're on a heading
    if not line:match("^#+ ") then
        vim.notify("Not on a heading line", vim.log.levels.WARN)
        return
    end

    vim.api.nvim_buf_set_lines(0, row, row, false, {
        "",
        "#properties",
        "id = ",
        "#end",
    })
    -- Place cursor on the id = line
    vim.api.nvim_win_set_cursor(0, { row + 3, 5 })
end

--- Demote heading (add one # level)
function M.demote_heading()
    local line = vim.api.nvim_get_current_line()
    if line:match("^#+ ") then
        vim.api.nvim_set_current_line("#" .. line)
    end
end

--- Promote heading (remove one # level)
function M.promote_heading()
    local line = vim.api.nvim_get_current_line()
    if line:match("^##+ ") then
        vim.api.nvim_set_current_line(line:sub(2))
    end
end

return M
