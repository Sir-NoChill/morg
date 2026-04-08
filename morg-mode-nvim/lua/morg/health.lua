--- Health check for morg-mode
local M = {}

function M.check()
    vim.health.start("morg-mode")

    -- Check binary
    local morg = require("morg")
    local binary = morg.binary()
    if vim.fn.executable(binary) == 1 then
        local version = vim.fn.system({ binary, "--version" })
        vim.health.ok("morg binary found: " .. binary .. " (" .. vim.trim(version) .. ")")
    else
        vim.health.error("morg binary not found: " .. binary, {
            "Install with: cargo install --path crates/morg-mode",
            "Or set binary path: require('morg').setup({ binary = '/path/to/morg' })",
        })
    end

    -- Check LuaSnip
    local ok, _ = pcall(require, "luasnip")
    if ok then
        vim.health.ok("LuaSnip available — morg snippets loaded")
    else
        vim.health.info("LuaSnip not found — snippets disabled (optional)")
    end

    -- Check capture config
    local capture_path = vim.fn.expand("~/.config/morg/capture.yaml")
    if vim.fn.filereadable(capture_path) == 1 then
        vim.health.ok("Capture config found: " .. capture_path)
    else
        vim.health.info("No capture config at " .. capture_path .. " (optional)")
    end
end

return M
