# Changelog

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
