--- Tests for morg command modules
describe("morg.commands", function()
    local command_modules = {
        "morg.commands.todos",
        "morg.commands.agenda",
        "morg.commands.tangle",
        "morg.commands.lint",
        "morg.commands.export",
        "morg.commands.search",
        "morg.commands.time",
        "morg.commands.columns",
        "morg.commands.capture",
        "morg.commands.refs",
        "morg.commands.archive",
        "morg.commands.id",
        "morg.commands.refile",
        "morg.commands.watch",
    }

    for _, mod_name in ipairs(command_modules) do
        it("loads " .. mod_name .. " without error", function()
            local ok, mod = pcall(require, mod_name)
            assert.is_true(ok, "Failed to load " .. mod_name)
            assert.is_table(mod)
            assert.is_function(mod.run, mod_name .. " missing run()")
        end)
    end
end)

describe("morg.commands (top-level)", function()
    it("loads and has setup function", function()
        local ok, cmds = pcall(require, "morg.commands")
        assert.is_true(ok)
        assert.is_function(cmds.setup)
    end)
end)
