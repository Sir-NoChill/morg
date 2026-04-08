--- Minimal init for morg-mode tests

vim.opt.rtp:append(".")
vim.cmd("runtime plugin/morg.lua")

-- Stub vim.notify for testing
_G._morg_test_notifications = {}
vim.notify = function(msg, level)
    table.insert(_G._morg_test_notifications, { msg = msg, level = level })
end
