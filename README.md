# ani2xcur

A command-line tool for installing Windows animated cursor schemes on Unix-like
operating systems running the [X Window System].

## Overview

Windows animated cursors use the [ANI file format], a container format that
stores multiple animation frames along with metadata such as frame timing and
frame order.

Related cursors are grouped into *cursor schemes*. A cursor scheme is typically
distributed together with an `Install.inf` file, which contains the metadata
required to register and install the scheme.

`ani2xcur` utilizes this information to accurately convert each animated cursor
into the [Xcursor] format and installs the resulting files in the locations
expected by the X Window System.

While many larger projects now parse INF files to perform bulk cursor
conversions, this project was the first to introduce that approach.

Originally created to fill that gap, it now focuses on providing an ergonomic
solution: a single command-line interface with intentionally designed
subcommands, optimized for speed and correctness.

## Installation

| Requirement | Version | Description                                  |
| :---------- | :------ | :------------------------------------------- |
| cargo       | 1.94.0  | Build and install the command-line interface |
| xcursorgen  | 1.0.8   | Xcursor generation backend                   |

Install from GitHub using `cargo`:

```bash
cargo install --git https://github.com/nicdgonzalez/ani2xcur
```

Or, download a pre-built binary from the [Releases] page on GitHub.

## Quickstart

From the directory containing the `Install.inf` file, run:

```bash
ani2xcur install --default-init
```

## Usage

> [!TIP]\
> Need a cursor to start with? Try NOiiRE's [Hornet Cursor] from Hollow Knight:
> Silksong.

From the directory containing the `Install.inf` file, run:

> [!TIP]\
> If your INF file has a different name, use the `--inf` flag instead of
> renaming the existing file.

```bash
ani2xcur init
```

This command parses the INF file and extracts the information needed to decode
each `.ani` file. The results are written to an intermediate `Cursor.toml`
file.

Next, build the cursor theme:

```bash
ani2xcur build
```

This command parses each `.ani` file and generates animated cursors in
**Xcursor** format. The cursors are placed in a theme directory using the
standard X cursor naming conventions.

Then, install the theme:

```bash
ani2xcur install
```

This creates the necessary links so X can locate and use the newly created
cursor theme.

Finally, enable the theme using your system's cursor settings. The exact
process varies by distribution, but most desktop environments provide a
command-line tool or a graphical settings panel.

Enjoy!

### Convert individual ANI files

If you only want to convert a single ANI file:

> [!NOTE]\
> This will output everything into a dedicated `build` directory like the
> `build` command does. I would recommend moving your ANI files into a separate
> directory prior to running this command.
>
> ```bash
> mkdir custom-theme
> mv *.ani ./custom-theme/
> cd custom-theme
> ```

```bash
ani2xcur convert Default.ani
```

## Roadmap

- [ ] Automatically scale cursors to standard sizes.
- [ ] Remove `xcursorgen` dependency.
- [ ] Remove need for `build` directory for the `convert` subcommand.

[ani file format]: https://en.wikipedia.org/wiki/ANI_(file_format)
[hornet cursor]: https://ko-fi.com/s/2e08ca3a58
[releases]: https://github.com/nicdgonzalez/ani-to-xcursor/releases
[x window system]: https://en.wikipedia.org/wiki/X_Window_System
[xcursor]: https://www.x.org/releases/current/doc/man/man3/Xcursor.3.xhtml
