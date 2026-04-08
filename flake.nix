{
  description = "morg-mode — markdown-idiomatic org-mode replacement";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "clippy" "rustfmt" ];
        };

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
        ];

        buildInputs = with pkgs; [
          openssl
        ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
          pkgs.darwin.apple_sdk.frameworks.Security
          pkgs.darwin.apple_sdk.frameworks.CoreFoundation
        ];

        morg = pkgs.rustPlatform.buildRustPackage {
          pname = "morg";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          inherit nativeBuildInputs buildInputs;
        };

        # LuaJIT-based environment for Neovim plugin tests.
        # All Lua packages are built against LuaJIT 2.1 (same ABI as Neovim),
        # avoiding the system Lua 5.x version mismatch.
        luaEnv = pkgs.luajit.withPackages (ps: with ps; [
          busted
          nlua
          luafilesystem
          luacheck
        ]);
      in
      {
        packages = {
          default = morg;
          morg = morg;
        };

        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;
          packages = with pkgs; [
            # Rust
            rustToolchain
            cargo-watch

            # Docs
            mdbook

            # Neovim plugin testing (LuaJIT — matches nvim runtime)
            neovim
            luaEnv

            # Linting
            stylua
          ];

          shellHook = ''
            echo "morg-mode dev shell"
            echo "  cargo build       — build morg binary"
            echo "  cargo test        — run Rust tests"
            echo "  cd morg-mode-nvim && busted . — run Lua tests"
            echo "  cd docs && mdbook serve       — preview docs"
          '';
        };

        # Minimal shell for CI Lua tests only
        devShells.lua-tests = pkgs.mkShell {
          packages = [
            pkgs.neovim
            luaEnv
          ];
        };
      });
}
