{
  description = "CK's stupid blog engine";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  };

  outputs = { self, nixpkgs, ... }:
  let
    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];

    forAllSystems = f:
      builtins.listToAttrs (map (system: {
        name = system;
        value = f system;
      }) systems);
  in {
    packages = forAllSystems (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        ck-clog = pkgs.stdenv.mkDerivation {
          pname = "ck-clog";
          version = "0.1";

          # Leave your directory structure:
          # src/main.c  (and whatever else you may add)
          src = ./src;

          buildPhase = ''
            $CC main.c -o ck-clog
          '';

          installPhase = ''
            mkdir -p $out/bin
            cp ck-clog $out/bin/
          '';
        };

        default = self.packages.${system}.ck-clog;
      });

    apps = forAllSystems (system: {
      default = {
        type = "app";
        # Correct binary path (was /bin/hello)
        program = "${self.packages.${system}.ck-clog}/bin/ck-clog";
      };
    });

    defaultPackage = self.packages;
    defaultApp = self.apps;

    devShells = forAllSystems (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        default = pkgs.mkShell {
          name = "ck-clog-dev";

          buildInputs = [
            # C Compiler
            pkgs.gcc 
          ];
        };
      });
  };
}

