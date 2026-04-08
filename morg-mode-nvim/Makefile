.PHONY: test lint format

test:
	eval "$$(luarocks path --bin --lua-version 5.1)" && busted .

lint:
	luacheck lua plugin spec

format:
	stylua lua plugin spec
