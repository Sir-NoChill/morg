--- morg-mode.nvim — Neovim integration for morg-mode
--- @module morg

local M = {}

--- Default configuration
--- @type MorgConfig
M.config = {
    --- Path to the morg binary. If nil, searches $PATH.
    binary = nil,
    --- Root directory for morg files. If nil, uses cwd.
    root = nil,
    --- File patterns to include
    patterns = { "*.md", "*.morg", "*.markdown" },
    --- Auto-tangle on save
    auto_tangle = false,
    --- Auto-lint on save (populate diagnostics)
    auto_lint = true,
    --- Keybinding prefix
    prefix = "<leader>m",
    --- Enable LuaSnip snippets
    snippets = true,
}

--- Setup the plugin with user configuration.
--- @param opts MorgConfig|nil
function M.setup(opts)
    M.config = vim.tbl_deep_extend("force", M.config, opts or {})
    M._binary = M.config.binary or "morg"

    -- Verify binary exists
    if vim.fn.executable(M._binary) ~= 1 then
        vim.notify("morg-mode: binary '" .. M._binary .. "' not found in PATH", vim.log.levels.WARN)
    end

    -- Register commands
    require("morg.commands").setup()

    -- Register keybindings
    require("morg.keymaps").setup(M.config.prefix)

    -- Register autocommands
    require("morg.autocmds").setup()

    -- Load snippets if enabled
    if M.config.snippets then
        -- Defer snippet loading until LuaSnip is available
        vim.defer_fn(function()
            local ok = pcall(require, "luasnip")
            if ok then
                require("morg.snippets").setup()
            end
        end, 100)
    end
end

--- Get the morg binary path.
--- @return string
function M.binary()
    return M._binary or "morg"
end

--- Get the root directory for morg operations.
--- @return string
function M.root()
    return M.config.root or vim.fn.getcwd()
end

--- Run a morg command and return stdout.
--- @param args string[] command arguments
--- @param callback fun(code: integer, stdout: string, stderr: string)|nil async callback
--- @return string|nil stdout (only in sync mode)
function M.run(args, callback)
    local cmd = vim.list_extend({ M.binary() }, args)

    if callback then
        -- Async
        local stdout_chunks = {}
        local stderr_chunks = {}
        vim.fn.jobstart(cmd, {
            stdout_buffered = true,
            stderr_buffered = true,
            on_stdout = function(_, data)
                if data then
                    vim.list_extend(stdout_chunks, data)
                end
            end,
            on_stderr = function(_, data)
                if data then
                    vim.list_extend(stderr_chunks, data)
                end
            end,
            on_exit = function(_, code)
                local stdout = table.concat(stdout_chunks, "\n")
                local stderr = table.concat(stderr_chunks, "\n")
                callback(code, stdout, stderr)
            end,
        })
        return nil
    else
        -- Sync
        local result = vim.fn.system(cmd)
        return result
    end
end

--- Run a morg command on the current file.
--- @param subcmd string
--- @param extra_args string[]|nil
--- @param callback fun(code: integer, stdout: string, stderr: string)|nil
function M.run_on_file(subcmd, extra_args, callback)
    local file = vim.fn.expand("%:p")
    local args = { subcmd }
    if extra_args then
        vim.list_extend(args, extra_args)
    end
    table.insert(args, file)
    return M.run(args, callback)
end

return M
