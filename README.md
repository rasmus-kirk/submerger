# Submerge

**Submerge** is a simple tool designed to merge subtitles from two different
files into a single output file. It's especially useful for language learners
who want to watch content with subtitles in multiple languages simultaneously.

The tool offers two main functionalities: Direct merging of two subtitle
files for quick, one-time use, and a more advanced recursive search feature
that scans entire directories for subtitle files matching specific language
preferences.

## Features

- Merge two subtitle files into one output file.
- Customize subtitle color and position for the second subtitle track.
- Recursively search directories for subtitle files to merge based on
  language markers.
- Supports both `.srt` and `.vtt` subtitle formats for inputs, only outputs
  `.srt` however.

## Installation

### Cargo

#### Installation

To install using cargo:

```bash
cargo install sub-merge
```

#### Building

One way to build this project is using [Rust](https://www.rust-lang.org/tools/install).

Clone the repository:

```bash
git clone https://github.com/rasmus-kirk/sub-merge.git
cd subtitle-merger
```

Build the project:

```bash
cargo build --release
```

### Nix

#### Installation

```bash
nix profile install github:rasmus-kirk/sub-merge
```

#### Building

This project has a nix flake, so it can be built using nix:

```bash
nix build github:rasmus-kirk/sub-merge
```

#### Running

If you just wish to run the program using nix:

```bash
nix run github:rasmus-kirk/sub-merge -- --help
```

#### Developement Shell

To enter a developement shell with the correct rust-toolchain and rust
analyzer using nix:

```bash
nix develop github:rasmus-kirk/sub-merge
```

If you use a shell other than bash:

```bash
nix develop github:rasmus-kirk/sub-merge -c $SHELL
```

## Usage

You can either merge two files directly or recursively search a directory
for matching subtitle pairs.

### 1. Merging Two Subtitle Files

Merge two subtitle files into a single output:

```
sub-merge simple [OPTIONS] <SUB1> <SUB2> <OUT>
```

Required:

- `<SUB1>`: Path to the first subtitle file
- `<SUB2>`: Path to the first subtitle file
- `<OUT>`:  Output file where the merged subtitles will be saved

Optional:

- `--color <COLOR>`          Sets the color for the second subtitle track
- `--position <POSITION>`    Sets the position of the second subtitle track (default: top-center)
- `--log-level <LOG_LEVEL>`  Sets the level of logging [default: warn] [possible values: error, warn, info, debug, trace]
- `--help`                   Print help

#### Example

```bash
sub-merge simple movie.en.srt movie.ja.srt --out merged.srt --color "#fbf1c7" --position top-center
```

### 2. Recursive Subtitle Merging

Search a directory for subtitle files matching given language codes and merge them:

```
sub-merge recursive [OPTIONS] <SUB1_LANG> <SUB2_LANG> <PATH>
```

Required:

- `<SUB1_LANG>`: Language code for the first subtitle file (e.g., `en` for English)
- `<SUB2_LANG>`: Language code for the second subtitle file (e.g., `ja` for Japanese)
- `<PATH>`:      Root directory to recursively search for subtitle files

Optional:

- `--out-ext <OUT_EXT>`:     The file extension for the output file (e.g. `file.en.srt` -> `file.merged.srt` if set to `merged.srt`) (Default: `srt`)
- `--vtt`:                   Also match and convert VTT files. Note, this will not output VTT files, only SRT is supported as output (Default: `true`)
- `--color <COLOR>`:         Sets the color for the second subtitle track
- `--position <POSITION>`:   Sets the position of the second subtitle track (Default: `top-center`)
- `--log-level <LOG_LEVEL>`: Sets the level of logging (Default: `warn`)

#### How it works

- The program reads both subtitle files and assigns different positions and
  colors to the second subtitle track (if configured).
- When merging recursively, the program looks for matching subtitle files
  based on the provided language codes (e.g., `en`, `ja`).
- If hearing-impaired subtitles are found (e.g., `en.hi`), they will be
  preferred only if normal subtitles (`en`) aren't available.
- The merged subtitle output file will contain both sets of subtitles and
  be written as `ORIGINAL_FILE_NAME.OUT_EXTENSION$` in the directory where
  the matching subs were found.

#### Example

```bash
sub-merge recursive en ja ./movies --color "#fbf1c7" --position top-center
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE.txt) file for details.
