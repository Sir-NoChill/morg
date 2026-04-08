--- LuaSnip snippet definitions for morg-mode
local M = {}

function M.setup()
    local ls = require("luasnip")
    local s = ls.snippet
    local t = ls.text_node
    local i = ls.insert_node
    local c = ls.choice_node
    local f = ls.function_node
    local fmt = require("luasnip.extras.fmt").fmt

    local date_now = function()
        return { os.date("%Y-%m-%d") }
    end
    local datetime_now = function()
        return { os.date("%Y-%m-%dT%H:%M") }
    end

    local morg_snippets = {
        -- Tags
        s("todo", fmt("#todo {}", { i(1, "task description") })),
        s("done", fmt("#done {}", { i(1, "completed task") })),
        s("deadline", fmt("#deadline {}", { f(date_now) })),
        s("scheduled", fmt("#scheduled {}", { f(date_now) })),
        s("date", fmt("#date {}", { f(date_now) })),
        s("event", fmt("#event {} {}", { f(date_now), i(1, "event description") })),
        s(
            "priority",
            fmt("#priority {}", {
                c(1, { t("A"), t("B"), t("C") }),
            })
        ),
        s("effort", fmt("#effort {}", { i(1, "1h") })),
        s("archive", t("#archive")),
        s("progress", t("#progress")),

        -- Time tracking
        s("clockin", fmt("#clock-in {}", { f(datetime_now) })),
        s("clockout", fmt("#clock-out {}", { f(datetime_now) })),
        s("clock", fmt("#clock {}", { i(1, "1h30m") })),

        -- Property drawer
        s(
            "props",
            fmt(
                [[
#properties
id = {}
{}
#end]],
                { i(1, ""), i(2) }
            )
        ),

        -- Code blocks with tangle
        s(
            "tangle",
            fmt(
                [[
```{} #tangle file={}
{}
```]],
                { i(1, "rust"), i(2, "output.rs"), i(3, "") }
            )
        ),

        -- Named code block
        s(
            "named",
            fmt(
                [[
```{} name={}
{}
```]],
                { i(1, "rust"), i(2, "block-name"), i(3, "") }
            )
        ),

        -- Noweb reference
        s("noweb", fmt("<<{}>>", { i(1, "block-name") })),

        -- Callout
        s(
            "note",
            fmt(
                [[
> [!note]
> {}]],
                { i(1, "content") }
            )
        ),

        s(
            "warn",
            fmt(
                [[
> [!warning]
> {}]],
                { i(1, "content") }
            )
        ),

        s(
            "tip",
            fmt(
                [[
> [!tip]
> {}]],
                { i(1, "content") }
            )
        ),

        -- Callout with metadata
        s(
            "cnote",
            fmt(
                [[
> [!{}][{}]
> {}]],
                { i(1, "note"), i(2, "#tangle file=out.txt"), i(3, "content") }
            )
        ),

        -- Frontmatter
        s(
            "fm",
            fmt(
                [[
---
title: {}
tags: [{}]
---
]],
                { i(1, "Title"), i(2, "") }
            )
        ),

        -- Frontmatter with todo sequences
        s(
            "fmtodo",
            fmt(
                [[
---
title: {}
todo_sequences:
  - [TODO, NEXT, WAIT, "|", DONE, CANCELLED]
---
]],
                { i(1, "Title") }
            )
        ),

        -- Checkbox items
        s("cb", fmt("- [ ] {}", { i(1, "task") })),
        s("cbx", fmt("- [x] {}", { i(1, "done task") })),

        -- Description list
        s("dl", fmt("- {} :: {}", { i(1, "term"), i(2, "description") })),

        -- Link with metadata
        s("mlink", fmt("[{}]({} [{}])", { i(1, "text"), i(2, "url"), i(3, "#tangle file=out") })),

        -- Footnote
        s("fn", fmt("[^{}]", { i(1, "1") })),
        s("fndef", fmt("[^{}]: {}", { i(1, "1"), i(2, "footnote text") })),

        -- Heading with properties
        s(
            "hprops",
            fmt(
                [[
## {}

#properties
id = {}
effort = {}
#end
]],
                { i(1, "Heading"), i(2, ""), i(3, "1h") }
            )
        ),

        -- Clock pair
        s(
            "clockpair",
            fmt(
                [[
#clock-in {}
#clock-out {}]],
                { f(datetime_now), i(1, "") }
            )
        ),
    }

    ls.add_snippets("markdown", morg_snippets)
    ls.add_snippets("morg", morg_snippets)
end

return M
