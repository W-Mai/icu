# Contributing to ICU

Thank you for improving ICU. This document records the repository's architecture boundaries, development workflow, and required validation steps.

## Before changing code

1. Read the relevant user and library documentation:
   - [`README.md`](README.md)
   - [`icu_lib/README.md`](icu_lib/README.md)
   - [`CHANGELOG.md`](CHANGELOG.md) for recent behavior
2. Inspect a sibling implementation before designing a new path.
3. Trace the complete interaction chain before editing. Compilation alone does not prove that a format or viewer feature is wired correctly.
4. Keep the change focused. Do not reformat unrelated files or combine cleanup with a functional change.

## Architecture

The workspace contains two packages with distinct responsibilities.

### `icu_tool`

The root package builds the `icu` executable.

- [`src/main.rs`](src/main.rs) selects the native CLI or WebAssembly viewer entry point.
- [`src/arguments.rs`](src/arguments.rs) defines the `clap` command surface and argument validation.
- [`src/cli.rs`](src/cli.rs) dispatches CLI commands and performs filesystem orchestration.
- [`src/converter.rs`](src/converter.rs) maps CLI-visible formats and options to `icu_lib` types.
- [`src/image_viewer`](src/image_viewer) contains the `egui`/`eframe` application, state model, rendering, and panels.
- [`locales`](locales) contains all user-visible viewer strings.

The root package may coordinate files, UI state, and user interaction. It should not duplicate format parsing or encoding logic owned by `icu_lib`.

### `icu_lib`

The library owns reusable decoding, encoding, intermediate data, and image post-processing.

- [`icu_lib/src/endecoder`](icu_lib/src/endecoder) contains format implementations.
- [`icu_lib/src/midata`](icu_lib/src/midata) defines the shared `MiData` model.
- [`icu_lib/src/postprocess`](icu_lib/src/postprocess) contains reusable transformations.

The normal data flow is:

```text
input bytes -> EnDecoder::decode -> MiData -> EnDecoder::encode -> output bytes
```

`EnDecoder` implementations should keep format-specific knowledge inside `icu_lib`. A new format should use an existing `MiData` variant where the semantics match. Add a new variant only when the data cannot be represented correctly by the current RGBA, grayscale, vector scene, font, or indexed-image models.

## Change checklists

### Adding or changing a format

Compare the change with an existing neighboring format and verify every applicable link:

- Implement or update `EnDecoder::can_decode`, `decode`, `encode`, and `info`.
- Register automatic detection in `icu_lib/src/endecoder/mod.rs` when the format is auto-detectable.
- Map CLI format names, file extensions, color formats, compression, and versions in `src/converter.rs`.
- Add or update `clap` validation in `src/arguments.rs`.
- Confirm CLI decoding and output dispatch in `src/cli.rs`.
- Confirm viewer loading, rendering, metadata, conversion, and export paths.
- Check native and `wasm32` behavior where file access or downloads differ.
- Add focused round-trip, metadata, malformed-input, or compatibility tests.
- Update README examples and user-visible help when the public contract changes.

Do not advertise a format merely because its enum value exists. Confirm that the encoder, decoder, and relevant output category are implemented.

### Changing the CLI

- Keep `src/arguments.rs`, `src/cli.rs`, and `src/converter.rs` consistent.
- Verify required and conditional options, value ranges, defaults, and output extension behavior.
- Run `cargo run --quiet -- --help` and `cargo run --quiet -- <command> --help`.
- Test overwrite handling, directory traversal, output folders, and `--stdout` when applicable.
- Update shell completion expectations when commands or options change.
- Update `README.md` without copying a full help snapshot that will quickly become stale.

### Changing the viewer

Trace state from input to cleanup:

- File picker, drag-and-drop, command shortcut, or WebAssembly file input.
- Decode and model construction.
- Sidebar selection and mode transitions.
- Central rendering and right-panel controls.
- Texture or render cache keys and invalidation.
- Diff state, selection state, and persisted-state migration or reset.
- Native save dialogs and browser downloads.
- Removal, replacement, and shutdown paths.

All user-visible strings belong in both [`locales/en-US.yml`](locales/en-US.yml) and [`locales/zh-CN.yml`](locales/zh-CN.yml). Avoid hardcoded labels in UI code.

### Changing `MiData`

A `MiData` change affects every consumer of the enum. Search for all matches and verify:

- Every encoder and decoder.
- CLI metadata and conversion behavior.
- Viewer loading and sidebar item construction.
- Rendering, conversion, export, and diff dispatch.
- Serialization boundaries in MIRX or other structured formats.
- Tests for each affected variant.

Avoid wildcard matches that silently hide an unhandled new variant.

## Development setup

The repository pins Rust in [`rust-toolchain.toml`](rust-toolchain.toml). A normal checkout installs the required `rustfmt` and `clippy` components through `rustup`.

```shell
git clone https://github.com/W-Mai/icu.git
cd icu
cargo build --workspace
```

Run the native application with:

```shell
cargo run -- show
```

Build the browser application with Trunk:

```shell
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk build
```

Use `trunk serve` for local browser development.

## Required quality gate

Run the complete gate before requesting review:

```shell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Also run the validation specific to the change:

- CLI changes: exercise the affected command and inspect its generated help.
- Library format changes: add and run representative decode, encode, and round-trip tests.
- Viewer changes: compare the full event/state/render/export chain with a sibling feature.
- Web changes: run `trunk build --release` when Trunk and the WASM target are available.

If a platform-specific gate cannot be run locally, state exactly what was not run and why.

## Tests

Keep tests close to the implementation when they exercise internal behavior. Use representative fixtures for binary formats, but avoid adding large generated files when a small deterministic fixture is sufficient.

A useful format test should verify behavior, not only that decoding returns a value. Depending on the format, assert dimensions, metadata, color conversion, stride, chunk type, glyph metrics, path operations, or round-trip equivalence.

## Documentation and public contracts

Update documentation in the same change when modifying:

- Commands, options, defaults, or validation.
- Supported input or output formats.
- Installation, build, or release requirements.
- Public `icu_lib` APIs or examples.
- Viewer workflows visible to users.

Keep documentation concise and source-backed. Prefer stable examples plus a reference to `--help` over pasted command output. Do not claim support until the complete implementation path is verified.

Record user-visible changes in [`CHANGELOG.md`](CHANGELOG.md) when preparing a release or when the maintainers request it. Do not rewrite existing release history.

## Pull requests

A focused pull request should include:

- The problem and root cause.
- The chosen approach and any compatibility implications.
- Files and interaction paths affected.
- Tests added or updated.
- Quality-gate commands and results.
- Any platform or WebAssembly checks not run.

Do not commit generated build output from `target/`, `dist/`, or `public/` unless the repository's release process explicitly requires it.
