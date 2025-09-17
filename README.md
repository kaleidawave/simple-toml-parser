# TOML parser/lexer

[![crates.io badge](https://img.shields.io/crates/v/simple-toml-parser?style=flat-square)](https://crates.io/crates/simple-toml-parser)
[![docs.rs badge](https://img.shields.io/docsrs/simple-toml-parser?style=flat-square)](https://docs.rs/simple-toml-parser/latest)

Features
- Under < 1k LOC Rust lexer (+ no dependencies)

See examples and tests for usage.

### Testing

```sh
spectra check ./specification.md "./target/debug/examples/parse --rpc --interactive"
```

### Formatter

There is an included formatter

You can use it in GitHub actions with the following

```yml
- uses: kaleidawave/release-downloader@improvements
  with:
    items: kaleidawave/simple-toml-parser@canary[format]
  
- name: Check 'Cargo.toml' formatting
  run: format Cargo.toml --check
```

### Links

- <https://toml.io/en/v1.0.0>
