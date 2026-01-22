# ANI to Xcursor

A command-line tool for converting Windows animated cursors to Linux.

## Installation

| Requirements | Version | Reason                     |
| :----------- | :------ | :------------------------- |
| cargo        | 1.87.0  | Build and install the CLI  |
| xcursorgen   | 1.0.8   | Xcursor generation backend |

Install from Git using cargo:

```bash
cargo +nightly install --git https://github.com/nicdgonzalez/ani-to-xcursor
```

If you don't have `nightly` installed, you can use rustup to get it:

```bash
rustup toolchain install nightly
```

## Quickstart

> [!TIP]\
> Need a cursor to start with? Try NOiiRE's [Hornet Cursor] from Hollow Knight:
> Silksong.

From the directory containing the `Install.inf` file, run:

```bash
ani-to-xcursor install

# For convenience, you can also pass the target directory via `--directory`:
# NOTE: This flag is available for all of the subcommands.
ani-to-xcursor install --directory /path/to/target/directory
```

This:

- Generates `Cursor.toml`
- Extracts frames from the ANI file
- Builds the Xcursor theme
- Installs it onto your system
- Prints the command required to activate the theme

## Usage

From the command line, navigate to the directory containing the `Install.inf`
file, then run the following command:

```bash
ani-to-xcursor init
```

This parses `Install.inf` and produces a `Cursor.toml` file.

Then, build the cursors:

```bash
ani-to-xcursor build
```

Finally, install the theme:

```bash
ani-to-xcursor install
```

The three commands are exposed separately for more control over the individual
steps.

Note, the `install` command will run all previous commands automatically if
needed.

## How it works

A cursor package on Windows typically contains a file called `Install.inf`.
This is a configuration file that tells Windows how to load the cursors. This
project uses that information to generate the necessary files on Linux.

This is the first (and as of writing, also the only) ANI to Xcursor project to
parse the INF file and automate the process from start to finish.

The cursor conversion process has three steps:

1. Parse `Install.inf` to generate a `Cursor.toml` file.
1. Read `Cursor.toml` to produce the `build` directory (containing the
   extracted frames, xcursorgen configuration files, cursor theme, etc.).
1. Create a symbolic link from `build/theme` into `$XDG_DATA_HOME/icons`,
   installing the cursor theme for the current user.

All files are built into the current directory, making it easy to delete and
undo all changes. The resulting directory structure should look something like
this:

```
.
├── build
│   ├── frames
│   └── theme
│       ├── cursors
│       └── index.theme
├── [...ANI]
├── Install.inf
└── Cursor.toml
```

[hornet cursor]: https://ko-fi.com/s/2e08ca3a58
