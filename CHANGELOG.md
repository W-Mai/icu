# Changelog

## [v0.9.0] - 2026-08-27

- ✨ Add explicit single-file and all-files Viewer export actions for static images, Groups, and animation frames.
- ✨ Recursively import native folders and WebAssembly-selected or dropped directories while preserving stable relative paths.
- ✨ Download WebAssembly batch exports as a ZIP archive and protect native batch output from overwrites and symlink traversal.
- 🐛 Reject static APNG conversion instead of writing ordinary PNG bytes with an `.apng` suffix.
- 🐛 Keep automatic sequence grouping stable when one batch contains multiple independent filename sequences.
- 🎨 Stack the Viewer conversion actions as full-width rows and show the resolved task count on `Convert All`.
- 🐛 Restore Web multi-file picking as a separate action from folder picking and capture recursive folder drops before the canvas consumes them.
- 🐛 Allow indexed images to participate in pixel diff against indexed and RGBA images.

## [v0.8.1] - 2026-08-26

- ✨ Add PNG color mode/compression and JPEG quality/background controls to the CLI and viewer.
- ✨ Import and export lossless animated WebP in the Viewer on native and WebAssembly.
- ⚠️ Extend `EncoderParams` with PNG/JPEG fields; external struct literals must add them or use `..Default::default()`.
- 🐛 Convert RGBA images to RGB inside the JPEG encoder instead of panicking on unsupported color data.
- 🐛 Convert indexed sources through their RGBA view for LVGL true-color and LZ4 exports, and reject invalid Viewer LVGL settings explicitly.
- 🐛 Preserve straight RGBA colors when the Viewer exports semi-transparent indexed images.

## [v0.8.0] - 2026-08-25

### 🚀 New Features

- ✨ Export grouped and imported GIF animations with configurable frame intervals and repeat counts.
- ✨ Export animated image groups as APNG with per-frame timing and repeat support.
- ✨ Select and export individual frames from imported GIF and APNG files.
- ✨ Show frame thumbnails for imported GIF/APNG animations and grouped image sequences.
- ✨ Add Cmd/Ctrl+A list selection, focused Up/Down navigation, and consistent Toggle controls for playback settings.
- ✨ Add workspace-wide numeric sequence grouping across mixed filename digit widths.
- ✨ Add indexed-image export parameter panels for LVGL, MIRX, and PNG workflows.

### 🎨 Improvements

- 🎨 Localize playback labels and frame information in the viewer.
- 🎨 Synchronize converter animation intervals with preview playback.
- 🎨 Refresh README branding, badges, navigation, and viewer snapshot presentation.

### 🐛 Fixes

- 🐛 Use a WASM-safe monotonic animation clock instead of `std::time::Instant`.
- 🛡️ Add GIF/APNG animation round-trip coverage and preserve frame timing during export.
- 🛡️ Add LVGL v9 raw-block LZ4 compression and decompression using `lz4_flex`.
- 🛡️ Validate compressed sizes and truncated LVGL headers without panicking.
- 🔄 Convert indexed LVGL inputs through their RGBA view when writing common output formats.

## [v0.7.0] - 2026-07-26

### 🔖 Version Tag

- 🚀 **New Features**:
    - ✨ **Full glyph parsing** — FreeType fonts now parse all glyphs from the cmap subtable instead of capping at 512. Fonts with thousands of glyphs are fully supported.
    - ✨ **Grid lazy loading** — Glyph grid renders only visible rows plus a 500-glyph prefetch buffer, with scroll-to-top on font switch. Large fonts load instantly without lag.
    - ✨ **Atlas on demand** — Atlas view no longer auto-renders on load. A "Render Atlas" button triggers rendering, avoiding startup cost for large fonts.
    - ✨ **Grid as default mode** — Selecting a font now opens in Grid mode instead of Atlas, matching the most common use case.
    - ✨ **Glyph diff unified diff mode** — Two fonts selected as diff1/diff2 auto-enter glyph diff with character input and three-panel canvas (A | diff | B) with synced zoom.
    - ✨ **MIRX scene export for glyph** — Glyphs can export to MIRX scene (VECTOR chunk) or image flat format.
    - ✨ **Rendered mode zoom/pan** — Font rendered preview supports zoom, pan, and double-click reset.
    - ✨ **Web file open** — `＋` button, empty canvas click, and `⌘O` work on web via synchronous `<input type="file">`.

- 🎨 **Improvements**:
    - Diff overlay alpha blending fixed to prevent black shadows on transparent backgrounds.
    - Identical glyph renders treated as zero diff instead of failing.
    - Diff hint messages differentiated by mode (items vs fonts vs same-type).
    - Grid columns dynamically computed from available width with vertical-only scroll.

- 🐛 **Fixes**:
    - FreeType glyph rendering uses outline bbox to prevent clipping of italic/descender characters.
    - Sequence aggregation skips font/path/indexed items to prevent false grouping.
    - Diff state (`diff_active`/`only_show_diff`) reset on startup to prevent stale FeedMe blank screen.
    - Glyph grid texture dimensions match actual rendered image to prevent panic.

## [v0.6.0] - 2026-07-20

### 🔖 Version Tag

- 🚀 **New Features**:
    - ✨ **Web file open** — `＋` button, empty canvas click, and `⌘O` now work on web via synchronous `<input type="file">` (bypasses wasm user-gesture restriction that blocked `rfd::AsyncFileDialog` in `spawn_local`). Native path unchanged.
    - ✨ **Glyph convert** — glyphs export to SVG (PathCmd → SVG path), PNG/JPEG/.../LVGL/MIRX (rasterize outline → RGBA → existing encode path). Convert tab shows full format dropdown for glyph items.
    - ✨ **FreeType font preview** — `draw_font_info_section` FreeType branch now renders preview text via new `render_freetype_text` (was empty before; otf/ttf fonts had no preview input).
    - ✨ **Glyph sidebar indent** — opened glyph items indent 16px under their parent font, with peach left bar marker keeping original position.
    - ✨ **New file focus** — adding files (via any method) selects the first new item instead of staying on the old selection.
    - ✨ **Glyph vector double-click reset** — double-click on glyph vector canvas resets zoom/pan to fit.
    - ✨ **⌘E for glyphs** — `⌘E` on a glyph switches to Convert tab (matching RGBA behavior).

- 🐛 **Fixes**:
    - Diff state (`diff_active` / `only_show_diff`) no longer persists across sessions — reset on startup so empty canvas shows FeedMe instead of a blank diff view.

## [v0.5.0] - 2026-07-19

### 🔖 Version Tag

- 🚀 **New Features**:
    - ✨ **Egui 0.35 migration** — upgraded from egui 0.33 + egui_plot 0.34 to egui 0.35 + egui_plot 0.36; dropped `catppuccin-egui` in favor of an inline MOCHA/LATTE palette with `theme::apply` setting both `dark_style` and `light_style` so toggles switch instantly.
    - ✨ **Three-column layout** — sidebar (260px) + main canvas + right inspector (300px), matching the original HTML prototype. Removed the extra `content_side` panel; per-mode controls migrated into the right panel's Info/Convert tabs.
    - ✨ **Unified right panel with Info/Convert/Diff tabs** — type-specific metadata, convert options, and diff controls all live in one right panel. Tab switch auto-manages `diff_active` (selecting Diff enables diff mode, switching away disables it).
    - ✨ **Glyph vector view** — FreeType glyphs render as true vector outlines with Bezier handles, baseline/bearing/advance annotations, zoom/pan (mouse wheel + drag), and touchpad pinch support. SDF/Gray atlas glyphs show approximate marching-squares contours with an "approximate" label.
    - ✨ **Glyph double-click → sidebar** — double-clicking a glyph in the grid opens it as a standalone `OpenedGlyph` sidebar item with its outline and metrics, auto-switching to vector view.
    - ✨ **Font bake charset 3-tab input** — Text (textarea) / Unicode Range (parser) / File (picker), with bit-depth validation (SDF 4/8, Gray 1/2/4/8).
    - ✨ **MIRX convert detailed params** — Pixel Format dropdown, Dithering toggle + level slider (NeuQuant sample factor 1-30), Stride Align input.
    - ✨ **Indexed Index Map view** — pre-rendered index-value-encoded texture with zoom overlay.
    - ✨ **⌘E export shortcut** — dispatches export by current item type (RGBA → Convert tab, Path/Indexed → PNG save dialog, Glyph → no-op).
    - ✨ **Full i18n** — all hardcoded UI strings translated (en-US / zh-CN). ~90 new locale keys covering section titles, buttons, tabs, labels, and hints.

- 🎨 **Visual Polish**:
    - Underline-style right panel tabs, section cards with visible borders, light theme palette realigned with HTML prototype, theme-aware central panel background.

- 🐛 **Fixes**:
    - Right panel squeeze when switching Info/Convert tabs resolved by consolidating side controls into the right inspector panel.
    - Font mode stuck in Vector after selecting an image item — `font_mode` resets to Atlas on image selection.
    - Diff button couldn't be toggled off — now a proper toggle linked to right tab state.

## [v0.4.0] - 2026-07-13

### 🔖 Version Tag

- 🚀 **New Features**:
    - ✨ **SVG gradient import/export** — `usvg::Paint::LinearGradient` / `RadialGradient` convert to `mirx::Paint::LinearGradient` / `RadialGradient` with stops, spread mode, gradient units, transform. SVG export emits `<linearGradient>` / `<radialGradient>` defs + `url(#gradN)` fill references. Pattern paints still dropped (no mirx Pattern variant).
    - ✨ **SVG clipPath import/export** — `<clipPath>` elements convert to `SceneOp::PushClip` / `PopClip` with `ResourceRef::Inline(Path)`. SVG export emits `clip-path="url(#clipN)"` attributes.
    - ✨ **SVG filter wire** — `<filter>` primitives (feGaussianBlur, feColorMatrix) encode into `GroupBegin.filter` as a semicolon-separated `ResourceRef::Token` string (`blur:<sx>:<sy>` / `cm:<matrix|saturate|hueRotate|luminance>`). SVG export emits `filter="url(#filterN)"`.
    - ✨ **SVG `<text>` import** — usvg text feature enabled; `<text>` / `<tspan>` flatten to glyph outline paths via `usvg::Tree::flattened()`. Font size / family / weight resolved by usvg.
    - ✨ **SVG `<image>` import** — `<image href="...">` embeds raster data as a placeholder rect (usvg doesn't expose pixel data to the tree walker).
    - ✨ **stroke-dasharray import/export** — `usvg::Stroke::dasharray()` maps to `StrokePath.dash` (Cow); SVG export emits `stroke-dasharray="a,b,c"` when dash is non-empty.
    - ✨ **NonZero fill-rule** — `usvg::FillRule::NonZero` passes through to `SceneOp::FillPath { fill_rule: NonZero }` instead of being coerced to EvenOdd.
    - ✨ **Font viewer glyph grid** — per-glyph thumbnail grid mode; click a glyph to inspect its outline path. Glyph diff mode overlays two atlases from different fonts.
    - ✨ **Indexed image export** — save edited indexed data (palette + per-pixel indexes) back to PNG (palette-encoded) or LVGL binary (I1/I2/I4/I8). Dither slider + palette edit panel + merge panel for multi-font bundles.
    - ✨ **Path viewer op inspector** — right panel shows selected SceneOp's fields (path cmds, paint, transform, fill_rule, stroke params).
    - ✨ **Multi-font bundle** — `FontData::MirxBundle` variant + bundle selector in font viewer; `icu merge-fonts` CLI merges single-font `.mirx` files.
    - ✨ **Theme-aware font rendering** — atlas tinted by theme fg color on bg, not raw grayscale; cache tint per theme, only re-render on theme switch.
    - ✨ **woff2 font decode** — `can_decode` detects woff2 magic; font info panel decompresses woff to ttf for metadata.
- 🔧 **Improvements**:
    - 🧹 **Atlas render via SwRenderer draw_label** — proper SDF sampling instead of raw byte threshold; ARGB8888 output uses `AlphaMode::Blend` with RGB forced to fg color for clean alpha edges.
    - 🧹 **mirui dependency bumped to v0.42.0** (crates.io) — `Paint` enum replaces `Color` in `FillPath`/`StrokePath`; `stops`/`dash` fields move `Vec` → `Cow<'static, [...]>` for const construction. ICU wraps runtime-built stops/dash in `Cow::Owned`.
- 🐛 **Bug Fixes**:
    - 🐛 Font atlas black edges fixed — SDF threshold-to-alpha conversion tints by coverage instead of hard threshold.
    - 🐛 Indexed hover stuck on theme switch — cache invalidated on theme change.
    - 🐛 Path highlight + diff conflict on simultaneous selection resolved.

### 📦 Dependencies

- mirui `0.41` → `0.42.0` (Paint enum, gradient/clip/blur, StrokePath cap/join/dash)

## [v0.3.0] - 2026-07-09

### 🔖 Version Tag

- 🚀 **New Features**:
    - ✨ **mirx image format support** — icu now decodes and encodes mirx FLAT images (all pixel formats: RGB565 / RGB565Swapped / RGB888 / RGBA8888 / BGRA8888 / XRGB8888) plus mirx CHUNK files with VECTOR / FONT chunks. `icu convert -F mirx` produces mirx output; `icu info` reports per-chunk metadata (op_count, glyph_count, source_size, bit_depth).
    - ✨ **TTF/OTF/TTC font parsing** via `ttf-parser` — `icu info` on a TTF reports family, style, units_per_em, ascender, descender, line_height, glyph_count. Glyph outlines collect into `mirx::PathCmd` (MoveTo / LineTo / QuadTo / CubicTo / Close) with 24.8 fixed coordinates.
    - ✨ **`icu bake-font` CLI** — bakes a TTF/OTF into a mirx FONT chunk (SDF or grayscale atlas). Pipeline: ttf-parser outline → mirui scanline coverage → Euclidean distance transform → 4/8-bit quantization (SDF) or packed coverage (gray). Supports `--charset` / `--charset-file` / `--size` / `--bit-depth` / `--spread` / `--format sdf|gray`.
    - ✨ **`icu merge-fonts` CLI** — merges multiple single-font `.mirx` files into one multi-FONT-chunk bundle for `MultiFontProvider` size-based selection at runtime.
    - ✨ **SVG import/export** — `icu info` / `icu show` / `icu convert` accept SVG. Import uses `usvg` (resolves `<defs>`, `<use>`, `<symbol>`, transform lists, style inheritance, gradients/clip-paths/masks). Export emits `<path>` / `<rect>` / `<line>` / `<g>` with fill, fill-opacity, fill-rule, stroke, stroke-width, stroke-linecap, stroke-linejoin, transform. `icu convert test.svg -F mirx` produces a VECTOR chunk.
    - ✨ **`MiData::PATH` / `FONT` / `INDEXED` variants** — `MiData` carries structured data alongside the flat RGBA preview. `PATH(SceneData)` wraps `mirx::Scene`; `FONT(FontData)` is an enum (`Mirx(mirx::Font)` | `FreeType(FreeTypeFontData)`); `INDEXED(IndexedImageData)` carries palette + per-pixel indexes + bpp.
    - ✨ **Font/path/indexed viewer panels** in the GUI — `ImageItem` carries the original `MiData`; central panel dispatches on variant. FONT shows metadata + preview-text input + rendered atlas; PATH shows a scene op tree; INDEXED shows a palette grid whose entries drive an `IndexHoverOverlay`.
    - ✨ **SDF → Gray downsample** — `sdf_to_gray_font` samples an SDF atlas at a target pixel size and packs coverage into an 8-bit grayscale font, no re-bake from TTF needed.
    - ✨ **Postprocess overlay framework** — `ImageOverlay` trait + `OverlayStack` composite with alpha blending and dirty-flag cache. `IndexHoverOverlay` highlights palette-indexed pixels; `QualityOverlay` renders a quantization-error heat map; `DiffOverlay` drives the diff display.
    - ✨ **LVGL indexed decode** — I1/I2/I4/I8 color formats decode to `MiData::INDEXED` with palette + indexes + bpp extracted before the generic RGBA conversion.
    - ✨ **ColorFormat unification** — `icu_lib::endecoder::ColorFormat` is the universal enum; LVGL and mirx each `From` their own wire format. `EncoderParams.color_format` is now `icu_lib::ColorFormat`.
- 🔧 **Improvements**:
    - 🧹 Diff display routed through `OverlayStack` + `DiffOverlay`. `ImageDiffResult` still feeds `diff_panel` pixel list / min-max range.
    - 🧹 `icu_lib` re-exports `mirx` and `image` crates.
    - 🧹 `MiData` + all variants derive `Clone` + `PartialEq`.

### 📦 Dependencies

- ➕ `ttf-parser = "0.25"` (icu_lib)
- ➕ `usvg = { version = "0.47", default-features = false }` (icu_lib; no fontdb, no text shaping — +~510KB binary)
- ➕ `mirx = "0.41"` + `mirui = { version = "0.41", default-features = false }` + `critical-section = { version = "1", features = ["std"] }` (icu_lib)

## [v0.2.0] - 2026-01-19

### 🔖 Version Tag

- 🚀 **New Features**:
    - ✨ Added a conversion panel in the GUI for image format conversion (supports LVGL, PNG, JPEG, etc.).
    - ✨ Shared conversion logic between CLI and GUI for consistent behavior.
    - ✨ Implemented automatic panel opening when a single image is loaded.
    - ✨ Added WASM support for image conversion and saving.
- 🔧 **Improvements**:
    - 🧹 Refactored `converter` module for better code reuse.
    - 🧹 Updated dependencies and fixed WASM compilation issues.

## [v0.1.23] - 2026-01-19

### 🔖 Version Tag

- 🚀 **New Features**:
    - ✨ Introduced i18n support with runtime language switching and auto-detection.
    - ✨ Added custom font support (Ark Pixel) for consistent UI rendering.
    - ✨ Added "Drag files here!" placeholder text for empty state.
    - ✨ Added bottom panel with version info and links.
- 🔧 **Improvements**:
    - 🧹 Moved language selector to bottom panel for better layout.
    - 🧹 Improved bottom panel responsiveness.
    - 🧹 Code formatting and clippy fixes in `icu_lib`.

## [v0.1.22] - 2026-01-19

### 🔖 Version Tag

- 🔧 **Refactor**: Renamed `image_shower` to `image_viewer` and restructured the module for better maintainability.
- 🚀 **New Features**:
    - Enhanced diff panel layout with grid presentation.
    - Added custom toggle component for improved UI interactions.
    - Added diff blend slider with quick selection buttons.
    - Implemented smooth scrolling to hovered diff pixel.
    - Added diff sorting and pagination controls.
    - Added support for hovered diff pixel highlighting.
- 💖 **UI Improvements**:
    - Improved "Image Diff" toggle placement.
    - Refactored diff panel UI for readability.
    - Improved plot highlighting and boundaries.

## [v0.1.21] - 2026-01-15

### 🔖 Version Tag

- 🚀 **New Features**: Added an Info Window to display image details (Width, Height, Format, Size, etc.).
- 🚀 **New Features**: Added "Tree View" for visualizing complex metadata (EXIF, PNG info) in YAML format.
- 🚀 **New Features**: Supported appending dropped images instead of replacing the current list.
- 🚀 **New Features**: Implemented metadata extraction for PNG (Color Type, Bit Depth, Interlace) and generic EXIF data.
- 🔧 **Improvements**: Refactored image processing and `ImageInfo` structure in `icu_lib`.

## [v0.1.20] - 2025-11-14

### 🔖 Version Tag

- 🚀 **New Features**: Added color difference visualization in `image_shower`.
- 🚀 **New Features**: Enhanced image diff display in `image_shower`.
- 🔧 **Improvements**: Refactored image diffing logic in `diff`.
- 🔧 **Improvements**: Moved diff panel to a side panel in `image_shower` for better UI.
- 🛠 **Refactoring**: Removed unused variables in `endecoder/utils`.
- 🔄 **Version Bump**: Version was bumped to 0.1.20 to reflect the updates.

## [v0.1.19] - 2025-08-22

### 🔖 Version Tag

- 🔧 **Improvements**: Updated deal_input_file_paths to handle stdout option and control flow.
- 🔧 **Improvements**: Bumped icu_lib version to 0.1.16 for compatibility and updates.
- 🔧 **Refactoring**: Refactored image diff logic to use icu_lib for improved performance and clarity.
- 🔧 **Refactoring**: Bumped version to 0.1.16 and updated RawImage methods to use parameters.
- 🔧 **Refactoring**: Restructured file tree, moved functions to cli mod.
- ✨ **New Features**: Added stdout option for outputting converted results directly to console.
- ✨ **New Features**: Added raw image header support and implemented RawImage struct for encoding/decoding.
- 💖 **Improvements**: Simplified image diff logic and improved variable naming.
- 💖 **Improvements**: Refactored image diff logic into utils module.
- 🎨 **Improvements**: Optimized color difference calculation and simplified blending logic.
- 🎨 **Improvements**: Added diff tolerance and min/max diff controls for improved image comparison.
- 🎉 **New Features**: Added initial project files for ICU Web UI with Nix flake and service worker.

## [v0.1.18] - 2025-08-01

### 🔖 Version Tag

- 🚀 **New Features**: Implemented options in `image_shower.rs` like showing only differences in diff mode, adding option
  to only show diff area and adjusting fast switch behavior, adding fast switch feature for diff mode with adjustable
  speed, and enhancing image blending logic and renaming diff_alpha to diff_blend.
- 📚 **Documentation**: Added components section for artifacts configuration in `oranda.json`.
- 📦 **Dependency Update**: Updated macOS platform configuration for eframe dependencies in `Cargo.toml`.
- 🔄 **Version Bump**: Bumped version to v0.1.18.

## [v0.1.17] - 2025-07-31

### 🔖 Version Tag

- 🧹 **Chores**: Update crate dependencies.
- 🔄 **Version Bump**: Version was bumped to 0.1.16 to reflect the updates and improvements.

## [v0.1.16] - 2025-07-31

### 🔖 Version Tag

- 🚀 **New Features**: Added diff support, supported alpha control, initialized diff image indices, adjusted inner and
  outer margins,
  enhanced image diff handling and selected image index, added image diff toggle and updated UI, refined position
  formatting in ImagePlotter, enhanced pixel rendering and added polygon support in ImagePlotter for pixel showing.
- 🎨 **Improvements**: Reorganized image selection logic in ImageShower for improved clarity and responsiveness.
- 🔄 **Version Bump**: Version was bumped to 0.1.16 to reflect the updates and improvements.

## [v0.1.15] - 2025-01-14

### 🔖 Version Tag

- 🔧 **Improvements**: Improved error handling for data size mismatches in lvgl. The logging statement was moved for
  correct execution, enhancing error reporting in image header processing.
- 🚀 **New Features**: Improved code formatting and organization in image_shower. Refactored the code for better
  readability, adjusted import statements and formatted button click event for setting background color with consistent
  indentation and line breaks.
- 🔄 **Version Bump**: Version was bumped to 0.1.15 to reflect the updates and improvements.

## [v0.1.14] - 2024-12-11

### 🔖 Version Tag

- 🚀 **New Features**: Added background color support to ImagePlotter, added unique ID to ImagePlotter, updated show
  command to handle multiple files, added image item selection and hover states, added new image plotting functionality,
  added image plotting functionality to Image Handling.
- 🔧 **Improvements**: Simplified image data conversion and update type references in ImageShower, refactored image data
  handling and update show method in Image Handling, simplified image selection logic in ImageShower, added parameter to
  `show_only` and update plot settings in ImagePlotter.
- 🐛 **Bug Fixes**: Fixed RLE decoding and handle empty image data in icu_lib.
- 🔄 **Version Bump**: Version was bumped to 0.1.14 to reflect the updates and improvements.

## [v0.1.13] - 2024-12-02

### 🔖 Version Tag

- 🚀 **New Features**: Added file drag and drop functionality to ImageShower, allowing users to easily drop files into
  the application for processing. (commit 89d234a4d57167e6e29138c1db39f8a7ede41ac4)
- 🚀 **New Features**: Enabled persistence of app state with `serde` serialization for `AppContext` struct, including
  settings like `show_grid` and `anti_alias`. (commit 7350fcec93f9c0c61ab49602be74dc951b2fca09)
- 🚀 **New Features**: Added anti-aliasing option to ImageShower, enhancing image quality with linear filtering when
  enabled. (commit c7571d70a7c6d865b2958aa6cbe400292042d5d2)
- 🚀 **New Features**: Implemented show grid option in ImageShower, allowing users to toggle the grid display for better
  visualization. (commit d697686281119190aac9395da7d3259858d4d0c1)
- 🔧 **Improvements**: Improved dropped files handling in ImageShower, accurately representing file information and
  preparing image data for display. (commit 589aa16ac7b916fc7c8e8a0d902553893b8de25c)
- 🔧 **Improvements**: Corrected typo in anti-aliasing toggle label and updated grid display settings for a cleaner
  look. (commit 783c82559e1928164995297d9450d52b7e628e2e)
- 🔧 **Improvements**: Simplified position checks in label formatter and improved image display with cursor interaction
  enhancements. (commit 693ccf55c3e6b6d208c8c8b6f90d43cf9e79dcfa)
- 🔧 **Improvements**: Updated grid display and coordinate formatting for precision, and removed unused imports to
  maintain code cleanliness. (commit 2aee817e8d5a29986ccee2802210d1471c67942b)
- 🛠 **Refactoring**: Refactored RLE encoding logic and LVGL handling, including updates to `RleCoder` and compression
  methods. (commits 2969fa94521a684868fc77adbc8cf325f1b8a381, 0b58b339c94806148e258aa8e1dff043c44df901)
- 🛠 **Refactoring**: Cleaned up icu_lib/src by removing unnecessary references and updating function calls for
  efficiency. (commit 59979684b79ad312af0cbff1185758c42d1775b8)
- 🐛 **Bug Fixes**: Fixed errors in image header stride handling and data size mismatches in icu_lib. (commit
  f63632b67e38a1d3e4f67827eba1c26a7b87380b)
- 📚 **Documentation**: Updated README files and added serialization details for better project understanding. (commits
  1996dfa999b0f68c295bce3b49a8a440c0317b1e, fde03acbf86b39e19a2537b401585da4a0b9ad40)
- 🔄 **Version Bump**: Version was bumped to 0.1.13 to reflect the updates and improvements.

## [v0.1.12] - 2024-11-08

### 🔖 Version Tag

- 🚀 **New Features**: support custom dither params, support 1 to 30 levels. 1 is the best level.
- 🔄 **Version Bump**: Version was bumped to 0.1.12 to reflect the updates and improvements.

## [v0.1.11] - 2024-05-01

### 🔖 Version Tag

劳动节快乐🎉
Happy Labor Day🎉

- 🚀 **New Features**: Added support for PNG indexes 1/2/4/8.
    - Now you can easily convert by using the `-C` option with `i1/2/4/8` color format.
- 🚀 **New Features**: Added support for Dither feature! By using `--dither` option you can make your pictures better and
  more natural.
- 🔄 **Version Bump**: Version was bumped to 0.1.11 to reflect the updates and improvements.

## [v0.1.10] - 2024-03-12

### 🔖 Version Tag

- 🚧 **Refactoring**: Refactored code to improve maintainability and readability.
- 🚧 **Refactoring**: Refactored error handling to improve user experience and reduce code complexity.
- 🚀 **New Features**: The way to display the path is more reasonable.
- 🚀 **New Features**: Added support for Auto-Complete feature for the command line interface. See `README.md` for more
  information.
- 🔄 **Version Bump**: Version was bumped to 0.1.10 to reflect the updates and improvements.

## [v0.1.9] - 2024-03-06

### 🔖 Version Tag

- 🚀 **New Features**: Added support for LVGL version 8 encode and decode.
- 🚀 **New Features**: Added support for image show for LVGL version 8.
- 🚀 **New Features**: Added support for more image information logging for LVGL version 8 and 9.
- 🔄 **Version Bump**: Version was bumped to 0.1.9 to reflect the updates and improvements.

## [v0.1.8] - 2024-03-04

### 🔖 Version Tag

- 🌍 **Oranda Updates**: Configurations were updated to improve the oranda module's functionality.
- 🐛 **Bug Fixes**: Web page bugs were addressed to enhance user experience.
- 🌐 **Webpage Additions**: GitHub Pages were added for better project documentation and visibility.
- 📦 **Dependency Updates**: Homebrew configurations were updated to ensure compatibility with the latest dependencies.
- 🚀 **New Features**: A new info command was added to the main module, and an API for image info retrieval was
  implemented in the icu_lib.
- 🛠 **CI/CD**: Automated build CI was added to streamline the development process.
- 📚 **Documentation**: README files were updated with more examples and detailed instructions.

## [v0.1.7] - 2024-03-03

### 🔖 Version Tag

- 📚 **Documentation**: README files were updated to provide more examples and clearer instructions.
- 🔄 **Dependencies**: Cargo dependencies were updated to the latest versions.
- 🔄 **Version Bump**: Version was bumped to 0.1.7 to reflect the updates and improvements.

## [v0.1.6] - 2024-03-03

### 🔖 Version Tag

- 🔄 **Code Refactoring**: Significant refactoring was done to improve the main module's codebase.
- 📁 **File Handling**: Enhanced support for file override and recursive conversion was added.
- 🔄 **Version Bump**: Version was bumped to 0.1.6 following the refactoring and feature additions.

## [v0.1.4] - 2024-02-29

### 🔖 Version Tag

- 📚 **README Updates**: README files were updated with new flags and detailed information about the icu tool.
- 🔄 **Version Bump**: The version was incremented to 0.1.4 after adding new features and making improvements.

## [v0.1.2] - 2024-02-26

### 🔖 Version Tag

- 📝 **Logging**: Enhanced logging was added to improve diagnostics and error handling.
- 🔄 **Dependencies**: Updated midata and enum parsing for the get_endecoder function.
- 🔄 **Version Bump**: The version was bumped to 0.1.2 to reflect the new features and fixes.

## [v0.1.1] - 2024-02-06

### 🔖 Version Tag

- 🚀 **Initial Release**: The first release of the project with basic functionality and initial documentation.
- 🖼️ **Image Support**: Added support for image_shower and various image formats.
- 🔧 **Argument Parsing**: Implemented basic argument parsing and added sub-commands for better user interaction.
- 🔄 **Dependencies**: Updated Cargo dependencies and prepared the project for publishing.

## [v0.1.0] - 2024-02-05

### 🔖 Version Tag

- 📄 **README Updates**: Initial README file was created with basic project information.
- 🔧 **Project Setup**: Set up the initial project structure and added basic functionality.
- 🔄 **Version Tag**: Tagged the initial release as version 0.1.0.
