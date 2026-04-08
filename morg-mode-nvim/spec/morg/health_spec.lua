--- Tests for morg health check
describe("morg.health", function()
    it("module loads and has check function", function()
        local ok, health = pcall(require, "morg.health")
        assert.is_true(ok)
        assert.is_function(health.check)
    end)
end)
