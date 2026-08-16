{ inputs, pkgs, ... }:

let
  rustBin = inputs.rust-overlay.lib.mkRustBin { } pkgs.buildPackages;
  msrvToolchain = rustBin.stable."1.74.0".minimal;
  nightlyToolchain = rustBin.nightly.latest.minimal;
in
{
  packages =
    [ pkgs.git ]
    ++ pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.cargo-tarpaulin ];

  languages.rust = {
    enable = true;
    channel = "stable";
  };

  tasks."ci:check" = {
    description = "Run the complete local CI suite";
    exec = ''
      cargo fmt --all --check
      cargo clippy --workspace --all-targets --all-features -- -D warnings
      RUSTDOCFLAGS="--cfg docsrs -D warnings" \
        cargo doc --workspace --no-deps --all-features --document-private-items
      cargo test --workspace --all-features
      RUSTC="${msrvToolchain}/bin/rustc" \
        RUSTDOC="${msrvToolchain}/bin/rustdoc" \
        "${msrvToolchain}/bin/cargo" test --workspace --all-features
      RUSTC="${nightlyToolchain}/bin/rustc" \
        RUSTDOC="${nightlyToolchain}/bin/rustdoc" \
        "${nightlyToolchain}/bin/cargo" test --workspace --all-features
      git diff --check
    '';
  };

  tasks."ci:coverage" = {
    description = "Generate the CI coverage report";
    exec = ''
      cargo tarpaulin --ignore-tests --workspace --all-features --out Xml -- \
        --test-threads 1
    '';
  };
}
