--- Tests for morg snippet definitions
describe("morg.snippets", function()
    it("module loads without error", function()
        -- Just verify the module can be required
        local ok, mod = pcall(require, "morg.snippets")
        assert.is_true(ok)
        assert.is_table(mod)
        assert.is_function(mod.setup)
    end)
end)
