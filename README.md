# ANI to Xcursor

A command-line tool for converting Windows animated cursors to Linux.

## Installation

| Requirements | Version | Reason                       |
| :----------- | :------ | :--------------------------- |
| cargo        | 1.87.0  | Build and install the CLI    |
| xcursorgen   | 1.0.8   | Xcursor generation backend   |
| python       | 3.13.7  | (Temporary) INF file parsing |

Install from Git using cargo:

```bash
cargo install --git https://github.com/nicdgonzalez/ani-to-xcursor
```

TL;DR

Temporary Python dependencies until the INF parser is re-implemented in Rust.

<details>
<summary>Read more</summary>

I help maintain a similar project, [win2xcursor], which is implemented in
Python; the INF file parser is also implemented in Python so I can share it
between the two projects. Until I re-implement it in Rust, the following Python
packages are also required.

</details>

```bash
python3 -m pip install tomli_w git+https://github.com/nicdgonzalez/inf.git
```

## Quickstart

From the directory containing the `Install.inf` file, run:

```bash
ani-to-xcursor install
```

This:

- Generates `Cursor.toml`
- Extracts frames from the ANI file
- Builds the Xcursor theme
- Installs it onto your system
- Prints the command required to activate the theme

To install and activate automatically in one go:

> [!NOTE]\
> Automatic activation is best-tested on GNOME (using `gsettings`). Commands
> for other Window Managers are defined in [src/commands/install.rs]. Pull
> requests to improve coverage are welcome!

```bash
eval "$(ani-to-xcursor install --skip-broken 2> /dev/null)"
```

## Usage

From the command line, navigate to the directory containing the `Install.inf`
file, then run the following command:

```bash
ani-to-xcursor init
```

This parses `Install.inf` and produces a `Cursor.toml` file.

> [!NOTE]\
> If this fails, your `Install.inf` may be missing or malformed. You can
> manually copy and edit the template: [`Cursor.toml`](./Cursor.toml).

Then, build the cursors:

```bash
ani-to-xcursor build
```

Finally, install the theme:

```bash
ani-to-xcursor install
```

The three commands are exposed separately so that advanced users can automate
or override individual steps.

The `install` command will run `init` and `build` automatically if needed.

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
undo all changes. The resulting directory structure:

```
Theme-Name
├── build
│   ├── frames
│   └── theme
│       ├── cursors
│       └── index.theme
└── Cursor.toml
├── Install.inf
├── [...ANI]
```

`Theme-Name` is the name of the directory containing the `Install.inf` file.
This becomes the final cursor theme name. (You can change this in `Cursor.toml`
if needed.)

As long as the ANI file names match the identifiers listed in `Install.inf`,
the application will locate and process them automatically. This is the most
tedious part of the process, so best efforts are made to find these files.

[src/commands/install.rs]: ./src/commands/install.rs
