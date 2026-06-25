# Contributing to wrongcl

Thanks for your interest in contributing. This guide covers the minimum you
need to land a change cleanly.

## Code of conduct

Participation in this project is governed by the
[Code of Conduct](CODE_OF_CONDUCT.md). By contributing you agree to uphold it.

## Reporting bugs and requesting features

Please use the issue templates under
[New issue](https://github.com/irrit-us/wrongcl/issues/new/choose). A small
reproducer (and the platform you saw it on) is worth more than a long
description.

For anything that may have security impact, do **not** open a public issue;
follow [SECURITY.md](SECURITY.md) instead.

## Development setup

The project has two layers:

* **Flutter** UI in `lib/` and `test/` — requires the Flutter SDK matching the
  version pinned in CI (`.github/workflows/ci.yml`).
* **Rust** core in `rust/` — requires a stable Rust toolchain.

Common commands before you push:

```bash
# Flutter side
dart format .
flutter analyze
flutter test

# Rust side (from rust/)
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

CI runs the same checks on Linux, macOS, Windows, Android, and iOS — keep your
diff green there before requesting review.

## Pull requests

* Keep changes focused. One concern per PR makes review and bisect easier.
* Match the existing style of the file you are editing; do not reformat
  adjacent code.
* Write a commit message that explains *why*, not just *what*. The recent
  `git log` is a good reference for the tone we aim for.
* If a change affects user-visible behavior, mention it in `CHANGELOG.md`.

## License

By contributing, you agree that your contributions will be licensed under the
[MIT License](LICENSE) that covers the project.
