# Ferrite Language Support

Official Visual Studio Code extension for the **Ferrite** programming language.

## Features

- **Syntax Highlighting**: Beautiful and semantic syntax highlighting for all Ferrite keywords, types, and language features.
- **Auto-Formatting**: Robust, built-in code formatter. Just press `Ctrl+S` (or format document) to automatically format your `.fe` files with correct indentation, spacing, and comment preservation.
- **Language Server**: Seamless integration with the Ferrite Language Server for real-time diagnostics and error reporting.

## Requirements

1. Install Ferrite from the [official repository](https://github.com/vishwanathdvgmm/ferrite).
2. That's it! The extension will automatically find `ferrite` if it is in your system's `PATH`, or if you are developing locally inside the Ferrite workspace. If it cannot be found, it will prompt you to configure the absolute path in the extension settings.

## Getting Started

1. Open a `.fe` file.
2. Start writing Ferrite code!
3. The Ferrite Language Server will automatically start and provide diagnostics.
4. Save your file to format it instantly.

## Known Issues

See the [issue tracker](https://github.com/vishwanathdvgmm/ferrite/issues) for known issues.

## Release Notes

### 1.1.0

- **Smart Compiler Discovery**: The extension now intelligently locates the `ferrite.exe` compiler from your system `PATH` or local workspace automatically.
- Removed redundant activation events for faster extension startup.
- Added user-friendly UI prompt for configuring the compiler path if missing.

### 1.0.0

- Initial release of the Ferrite VS Code extension.
- Added syntax highlighting grammar.
- Integrated Language Server with real-time diagnostics.
- Added Format on Save support.
