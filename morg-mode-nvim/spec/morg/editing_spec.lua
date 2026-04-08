--- Tests for morg editing helpers
describe("morg.editing", function()
    local editing

    before_each(function()
        package.loaded["morg.editing"] = nil
        editing = require("morg.editing")
    end)

    describe("toggle_checkbox", function()
        it("checks an unchecked box", function()
            -- Set up a buffer with a checkbox line
            local buf = vim.api.nvim_create_buf(false, true)
            vim.api.nvim_set_current_buf(buf)
            vim.api.nvim_buf_set_lines(buf, 0, -1, false, { "- [ ] Task item" })
            vim.api.nvim_win_set_cursor(0, { 1, 0 })

            editing.toggle_checkbox()

            local result = vim.api.nvim_buf_get_lines(buf, 0, 1, false)[1]
            assert.equals("- [x] Task item", result)

            vim.api.nvim_buf_delete(buf, { force = true })
        end)

        it("unchecks a checked box", function()
            local buf = vim.api.nvim_create_buf(false, true)
            vim.api.nvim_set_current_buf(buf)
            vim.api.nvim_buf_set_lines(buf, 0, -1, false, { "- [x] Done item" })
            vim.api.nvim_win_set_cursor(0, { 1, 0 })

            editing.toggle_checkbox()

            local result = vim.api.nvim_buf_get_lines(buf, 0, 1, false)[1]
            assert.equals("- [ ] Done item", result)

            vim.api.nvim_buf_delete(buf, { force = true })
        end)

        it("adds checkbox to plain list item", function()
            local buf = vim.api.nvim_create_buf(false, true)
            vim.api.nvim_set_current_buf(buf)
            vim.api.nvim_buf_set_lines(buf, 0, -1, false, { "- Plain item" })
            vim.api.nvim_win_set_cursor(0, { 1, 0 })

            editing.toggle_checkbox()

            local result = vim.api.nvim_buf_get_lines(buf, 0, 1, false)[1]
            assert.equals("- [ ] Plain item", result)

            vim.api.nvim_buf_delete(buf, { force = true })
        end)
    end)

    describe("demote_heading", function()
        it("adds a # level", function()
            local buf = vim.api.nvim_create_buf(false, true)
            vim.api.nvim_set_current_buf(buf)
            vim.api.nvim_buf_set_lines(buf, 0, -1, false, { "## Heading" })
            vim.api.nvim_win_set_cursor(0, { 1, 0 })

            editing.demote_heading()

            local result = vim.api.nvim_buf_get_lines(buf, 0, 1, false)[1]
            assert.equals("### Heading", result)

            vim.api.nvim_buf_delete(buf, { force = true })
        end)
    end)

    describe("promote_heading", function()
        it("removes a # level", function()
            local buf = vim.api.nvim_create_buf(false, true)
            vim.api.nvim_set_current_buf(buf)
            vim.api.nvim_buf_set_lines(buf, 0, -1, false, { "### Heading" })
            vim.api.nvim_win_set_cursor(0, { 1, 0 })

            editing.promote_heading()

            local result = vim.api.nvim_buf_get_lines(buf, 0, 1, false)[1]
            assert.equals("## Heading", result)

            vim.api.nvim_buf_delete(buf, { force = true })
        end)

        it("does not promote past h1", function()
            local buf = vim.api.nvim_create_buf(false, true)
            vim.api.nvim_set_current_buf(buf)
            vim.api.nvim_buf_set_lines(buf, 0, -1, false, { "# Top Level" })
            vim.api.nvim_win_set_cursor(0, { 1, 0 })

            editing.promote_heading()

            local result = vim.api.nvim_buf_get_lines(buf, 0, 1, false)[1]
            assert.equals("# Top Level", result)

            vim.api.nvim_buf_delete(buf, { force = true })
        end)
    end)

    describe("insert_tag", function()
        it("inserts a tag at cursor", function()
            local buf = vim.api.nvim_create_buf(false, true)
            vim.api.nvim_set_current_buf(buf)
            vim.api.nvim_buf_set_lines(buf, 0, -1, false, { "Some text" })
            vim.api.nvim_win_set_cursor(0, { 1, 9 }) -- end of "Some text"

            editing.insert_tag("todo")

            local result = vim.api.nvim_buf_get_lines(buf, 0, 1, false)[1]
            assert.truthy(result:match("#todo"))

            vim.api.nvim_buf_delete(buf, { force = true })
        end)
    end)
end)
