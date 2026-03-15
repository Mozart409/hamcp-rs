{
  description = "hamcp-rs - MCP server for Home Assistant";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    crane,
  }:
    (flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        config.allowUnfree = true;
        overlays = [rust-overlay.overlays.default];
      };
      rust = pkgs.rust-bin.nightly."2026-02-15".default.override {
        extensions = ["rustfmt" "clippy" "rust-src"];
      };

      craneLib = (crane.mkLib pkgs).overrideToolchain rust;

      src = craneLib.cleanCargoSource ./.;

      commonArgs = {
        inherit src;
        pname = "hamcp";
        version = "0.1.0";
        strictDeps = true;
        buildInputs =
          [pkgs.openssl]
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];
        nativeBuildInputs = [pkgs.pkg-config];
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      mcp = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;
          cargoExtraArgs = "--bin mcp";
        });
    in {
      packages = {
        default = mcp;
        inherit mcp;
      };

      devShells.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          # keep-sorted start
          actionlint
          bacon
          cargo-audit
          cargo-deny
          cargo-outdated
          cargo-workspaces
          cocogitto
          dbeaver-bin
          docker
          docker-buildx
          docker-compose
          just
          keep-sorted
          lazydocker
          lefthook
          nodejs_24
          opencode
          opentofu
          postgresql_16
          rust
          sqlx-cli
          tailwindcss_4
          trivy
          typos
          # keep-sorted end
        ];
        shellHook = ''
          lefthook install
          cog install-hook
          export COMPOSE_BAKE=true
        '';
      };
    }))
    // {
      nixosModules.default = {
        config,
        lib,
        pkgs,
        ...
      }: let
        cfg = config.services.hamcp;
      in {
        options.services.hamcp = {
          enable = lib.mkEnableOption "hamcp MCP server for Home Assistant";

          package = lib.mkOption {
            type = lib.types.package;
            default = self.packages.${pkgs.system}.default;
            defaultText = lib.literalExpression "self.packages.\${pkgs.system}.default";
            description = "The hamcp package to use.";
          };

          haUrl = lib.mkOption {
            type = lib.types.str;
            description = "Home Assistant instance URL.";
            example = "http://homeassistant.local:8123";
          };

          haTokenFile = lib.mkOption {
            type = lib.types.path;
            description = ''
              Path to a file containing the Home Assistant long-lived access token.
              Compatible with sops-nix, agenix, or any file-based secret manager.
            '';
            example = "/run/secrets/ha-token";
          };

          port = lib.mkOption {
            type = lib.types.port;
            default = 3000;
            description = "Port for the MCP server to listen on.";
          };

          address = lib.mkOption {
            type = lib.types.str;
            default = "0.0.0.0";
            description = "Bind address for the MCP server.";
          };

          openFirewall = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "Whether to open the firewall port for the MCP server.";
          };
        };

        config = lib.mkIf cfg.enable {
          systemd.services.hamcp = {
            description = "hamcp - MCP server for Home Assistant";
            wantedBy = ["multi-user.target"];
            after = ["network-online.target"];
            wants = ["network-online.target"];

            environment = {
              HA_URL = cfg.haUrl;
              MCP_ADDR = "${cfg.address}:${toString cfg.port}";
            };

            serviceConfig = {
              Type = "simple";
              Restart = "on-failure";
              RestartSec = 5;

              DynamicUser = true;
              LoadCredential = "ha-token:${cfg.haTokenFile}";

              # Security hardening
              NoNewPrivileges = true;
              ProtectSystem = "strict";
              ProtectHome = true;
              PrivateTmp = true;
              PrivateDevices = true;
              ProtectKernelTunables = true;
              ProtectKernelModules = true;
              ProtectControlGroups = true;
              RestrictSUIDSGID = true;
              RestrictNamespaces = true;
              LockPersonality = true;
              MemoryDenyWriteExecute = true;
              RestrictRealtime = true;
            };

            script = ''
              export HA_TOKEN="$(< "$CREDENTIALS_DIRECTORY/ha-token")"
              exec ${cfg.package}/bin/mcp
            '';
          };

          networking.firewall.allowedTCPPorts =
            lib.mkIf cfg.openFirewall [cfg.port];
        };
      };
    };
}
