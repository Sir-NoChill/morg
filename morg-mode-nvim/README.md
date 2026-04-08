# morg-mode.nvim

Neovim integration for [morg-mode](../), a markdown-idiomatic org-mode replacement.

Thin async wrapper around the `morg` CLI — all parsing, tangling, and reporting happens in the Rust binary. The plugin provides commands, keybindings, Telescope pickers, LuaSnip snippets, and editing helpers.

## Requirements

| Requirement | Version | Notes |
|---|---|---|
| Neovim | >= 0.12 | For `vim.pack.add()`. Use >= 0.9 with lazy.nvim/packer. |
| Rust toolchain | stable | To build the `morg` binary |
| `morg` binary | in `$PATH` | Built from the workspace root |
| [telescope.nvim](https://github.com/nvim-telescope/telescope.nvim) | any | Optional — for fuzzy pickers |
| [LuaSnip](https://github.com/L3MON4D3/LuaSnip) | any | Optional — for snippet expansion |

## Building the `morg` binary

From the workspace root (`morg-mode/`):

```sh
# Development build
cargo build

# Release build (recommended)
cargo build --release

# Install to ~/.cargo/bin (adds to $PATH)
cargo install --path crates/morg-mode
```

The binary is called `morg`. Verify with:

```sh
morg --help
```

## Installation

### vim.pack.add (Neovim >= 0.12, built-in)

```lua
-- In your init.lua

-- Install the plugin
vim.pack.add({
    "your-user/morg-mode-nvim",
})

-- Optional: telescope and LuaSnip
vim.pack.add({
    "nvim-telescope/telescope.nvim",
    "nvim-lua/plenary.nvim",
    "L3MON4D3/LuaSnip",
})

-- Build the morg binary on install/update
vim.api.nvim_create_autocmd("PackChanged", {
    callback = function(event)
        if event.data.spec.name == "morg-mode-nvim" then
            local morg_root = event.data.path
            -- If the Rust workspace is co-located (monorepo), build from parent
            -- Otherwise, instruct the user to build separately
            vim.notify("morg-mode-nvim installed. Build the binary with:\n  cargo install --path crates/morg-mode", vim.log.levels.INFO)
        end
    end,
})

-- Setup
require("morg").setup({
    -- All options are optional. Defaults shown.
    binary = nil,           -- Path to morg binary. nil = search $PATH
    root = nil,             -- Root directory for morg files. nil = cwd
    patterns = { "*.md", "*.morg", "*.markdown" },
    auto_tangle = false,    -- Tangle on save
    auto_lint = true,       -- Lint on save (populates diagnostics)
    prefix = "<leader>m",   -- Keybinding prefix
    snippets = true,        -- Load LuaSnip snippets (if LuaSnip available)
})
```

### lazy.nvim

```lua
{
    "your-user/morg-mode-nvim",
    ft = { "markdown", "morg" },
    dependencies = {
        "nvim-telescope/telescope.nvim",  -- optional
        "L3MON4D3/LuaSnip",              -- optional
    },
    build = function()
        -- Remind user to build the Rust binary
        vim.notify("morg-mode: run `cargo install --path crates/morg-mode` to build the binary")
    end,
    opts = {
        -- binary = "/path/to/morg",
        -- root = "~/notes",
        -- auto_tangle = true,
    },
}
```

### Local development (symlink)

If you're developing morg-mode locally:

```sh
# Build the binary
cd /path/to/morg-mode
cargo install --path crates/morg-mode

# Symlink the plugin into nvim's pack path
mkdir -p ~/.local/share/nvim/site/pack/dev/start
ln -s /path/to/morg-mode/morg-mode-nvim ~/.local/share/nvim/site/pack/dev/start/morg-mode-nvim
```

Then in `init.lua`:

```lua
require("morg").setup()
```

## Commands

All commands shell out to `morg` asynchronously — the editor never blocks.

| Command | Description |
|---|---|
| `:MorgTodos` | List all TODOs in quickfix list |
| `:MorgAgenda` | Show agenda in a split (`gd` to jump, `q` to close) |
| `:MorgTangle` | Tangle current file |
| `:MorgLint` | Lint current file (populates `vim.diagnostic`) |
| `:MorgExport [path]` | Export to HTML and open in browser |
| `:MorgSearch <query>` | Search across project files (quickfix) |
| `:MorgTime` | Time tracking report in a split |
| `:MorgColumns` | Column view of headings in a split |
| `:MorgCapture <template> <text>` | Quick capture using template |
| `:MorgRefs` | Validate cross-file `id:` references |
| `:MorgArchive` | Move `#archive` subtrees to archive files (`!` for dry run) |
| `:MorgId` | Assign UUIDs to headings without IDs (`!` for dry run) |
| `:MorgRefile <target>` | Move heading at cursor to target file/heading |
| `:MorgWatch [cmd]` | Toggle file watcher (default: tangle) |

## Telescope Pickers

Requires [telescope.nvim](https://github.com/nvim-telescope/telescope.nvim). All pickers consume `--format json` output from the `morg` binary.

| Picker | Description |
|---|---|
| `:Telescope morg todos` | Browse TODOs with status, priority, effort |
| `:Telescope morg agenda` | Browse agenda entries by date |
| `:Telescope morg search` | Prompted full-text search |
| `:Telescope morg tags` | Browse all unique tags |
| `:Telescope morg headings` | Browse all headings |

All pickers jump to source file:line on `<CR>`.

## Keybindings

### Global (prefix: `<leader>m`, configurable)

| Key | Action |
|---|---|
| `<leader>mt` | `:MorgTodos` |
| `<leader>ma` | `:MorgAgenda` |
| `<leader>mT` | `:MorgTangle` |
| `<leader>ml` | `:MorgLint` |
| `<leader>me` | `:MorgExport` |
| `<leader>ms` | `:MorgSearch` |
| `<leader>mr` | `:MorgTime` |
| `<leader>mc` | `:MorgColumns` |
| `<leader>mR` | `:MorgRefs` |
| `<leader>mw` | `:MorgWatch` |
| `<leader>mft` | `:Telescope morg todos` |
| `<leader>mfa` | `:Telescope morg agenda` |
| `<leader>mfs` | `:Telescope morg search` |
| `<leader>mfg` | `:Telescope morg tags` |
| `<leader>mfh` | `:Telescope morg headings` |

### Buffer-local (active in markdown/morg files)

| Key | Action |
|---|---|
| `<C-Space>` | Toggle checkbox (`[ ]` <-> `[x]`) |
| `<leader>it` | Insert `#todo` at cursor |
| `<leader>id` | Insert `#done` at cursor |
| `<leader>iD` | Insert `#deadline` with today's date |
| `<leader>iS` | Insert `#scheduled` with today's date |
| `<leader>ic` | Insert `#clock-in` with current time |
| `<leader>iC` | Insert `#clock-out` with current time |
| `<leader>ip` | Insert `#properties` block after heading |
| `>>` | Demote heading (add `#`) |
| `<<` | Promote heading (remove `#`) |

## LuaSnip Snippets

Loaded automatically when LuaSnip is available. Active in `markdown` and `morg` filetypes.

| Trigger | Expands to |
|---|---|
| `todo` | `#todo {description}` |
| `done` | `#done {description}` |
| `deadline` | `#deadline {today's date}` |
| `scheduled` | `#scheduled {today's date}` |
| `event` | `#event {today} {description}` |
| `priority` | `#priority A/B/C` (choice node) |
| `effort` | `#effort {duration}` |
| `clockin` | `#clock-in {current datetime}` |
| `clockout` | `#clock-out {current datetime}` |
| `clock` | `#clock {duration}` |
| `clockpair` | Clock in + clock out pair |
| `props` | `#properties` / `#end` block with `id` |
| `tangle` | Code fence with `#tangle file=` |
| `named` | Code fence with `name=` attribute |
| `noweb` | `<<block-name>>` reference |
| `note` | `> [!note]` callout |
| `warn` | `> [!warning]` callout |
| `tip` | `> [!tip]` callout |
| `cnote` | Callout with metadata bracket |
| `fm` | YAML frontmatter |
| `fmtodo` | Frontmatter with custom todo sequences |
| `cb` / `cbx` | Checkbox items (unchecked / checked) |
| `dl` | Description list item (`term :: description`) |
| `mlink` | Link with metadata `[text](url [#tangle file=])` |
| `fn` / `fndef` | Footnote reference / definition |
| `hprops` | Heading with properties block |

## Health Check

```vim
:checkhealth morg
```

Verifies:
- `morg` binary is found and reports its version
- LuaSnip availability
- Capture template config file

## Development

### Running tests

```sh
# Prerequisites (one-time)
luarocks --lua-version 5.1 install busted nlua --local

# Run Lua tests
cd morg-mode-nvim
eval "$(luarocks path --bin --lua-version 5.1)" && busted .

# Run Rust tests (from workspace root)
cd ..
cargo test --workspace
```

### Project structure

```
morg-mode-nvim/
  plugin/morg.lua              # Entry point (lazy guard)
  lua/morg/
    init.lua                   # Setup, config, async morg.run() helper
    commands.lua               # :Morg* command registrations
    commands/                  # 14 command implementations
    keymaps.lua                # Global + buffer-local keybindings
    editing.lua                # Checkbox toggle, tag insertion, heading promote/demote
    autocmds.lua               # Auto-lint, auto-tangle on save
    snippets/init.lua          # 30 LuaSnip snippet definitions
    health.lua                 # :checkhealth integration
  lua/telescope/_extensions/
    morg.lua                   # 5 Telescope pickers
  spec/morg/                   # 30 busted tests
```
