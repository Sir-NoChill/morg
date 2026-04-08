--- Tests for morg configuration and setup
describe("morg", function()
    local morg

    before_each(function()
        package.loaded["morg"] = nil
        package.loaded["morg.commands"] = nil
        package.loaded["morg.keymaps"] = nil
        package.loaded["morg.autocmds"] = nil
        morg = require("morg")
    end)

    describe("config", function()
        it("has sensible defaults", function()
            assert.equals(nil, morg.config.binary)
            assert.equals(nil, morg.config.root)
            assert.is_true(morg.config.auto_lint)
            assert.is_false(morg.config.auto_tangle)
            assert.equals("<leader>m", morg.config.prefix)
            assert.is_true(morg.config.snippets)
        end)

        it("merges user config", function()
            -- Simulate setup without triggering keymaps etc
            morg.config = vim.tbl_deep_extend("force", morg.config, {
                binary = "/usr/local/bin/morg",
                auto_tangle = true,
                prefix = "<leader>o",
            })

            assert.equals("/usr/local/bin/morg", morg.config.binary)
            assert.is_true(morg.config.auto_tangle)
            assert.equals("<leader>o", morg.config.prefix)
            -- Unchanged defaults preserved
            assert.is_true(morg.config.auto_lint)
            assert.is_true(morg.config.snippets)
        end)
    end)

    describe("binary", function()
        it("defaults to 'morg'", function()
            assert.equals("morg", morg.binary())
        end)

        it("uses configured binary", function()
            morg._binary = "/custom/path/morg"
            assert.equals("/custom/path/morg", morg.binary())
        end)
    end)

    describe("root", function()
        it("defaults to cwd", function()
            assert.equals(vim.fn.getcwd(), morg.root())
        end)

        it("uses configured root", function()
            morg.config.root = "/tmp/notes"
            assert.equals("/tmp/notes", morg.root())
        end)
    end)
end)
