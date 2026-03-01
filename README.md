# NAS Normalizer

A tui application written in rust to help normalize media directories.

## Tests:
```sh
cargo test
```

## Build:
```sh
cargo build
```

## Run:
```sh
nas_normlzr --help
```

```sh
Interactively normalize media paths on your NAS

Usage: nas_normlzr [OPTIONS]

Options:
  -c, --config <CONFIG>  Path to the TOML configuration file [default: config.toml]
  -d, --dry-run          Preview changes without renaming any files
      --movies <PATH>    Override the movies root directory from config
      --tv <PATH>        Override the TV shows root directory from config
  -h, --help             Print help
  -V, --version          Print version
```