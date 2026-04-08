.PHONY: test lint format

# Run tests. Works in nix devShell (busted on PATH) or with hererocks/luarocks.
test:
	@command -v busted >/dev/null 2>&1 && busted . \
		|| (eval "$$(luarocks path --bin --lua-version 5.1)" && busted .)

lint:
	luacheck lua plugin spec

format:
	stylua lua plugin spec
