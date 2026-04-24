# ani2xcur

> [!NOTE]\
> This project recieved a full rewrite and was renamed from `ani-to-xcursor` to
> `ani2xcur` on March 10, 2026.

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
solution: a single command-line interface with intentionally designed options
and subcommands, optimized for speed and correctness. See
[Benchmarks](#Benchmarks) for comparisons against similar projects.

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

> [!TIP]\
> Need a cursor to start with? Try NOiiRE's [Hornet Cursor] from Hollow Knight:
> Silksong.

From the directory containing the `Install.inf` file, run:

```bash
ani2xcur install --default-init
```

## Usage

From the directory containing the `Install.inf` file, run:

> [!TIP]\
> If your INF file has a different name, use the `--inf` flag instead of
> renaming the existing file.
>
> ```bash
> ani2xcur init --inf Other.inf
> ```

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

> [!NOTE]\
> Do NOT use this command if you are converting a cursor theme that does not
> have an INF file. Instead, use the `--skip-inf` flag on the `init` command to
> create a generic manifest that can be manually edited. This way, you can
> still use the `build` and `install` commands which do a lot of the heavy
> lifting.
>
> ```bash
> ani2xcur init --skip-inf
> ```

If you only want to convert a single ANI file:

> [!TIP]\
> This will output everything into a dedicated `build` directory like the
> `build` command does. I would recommend moving your ANI files into a separate
> directory prior to running this command. (This step won't be necessary in a
> future update.)
>
> ```bash
> mkdir custom-theme
> mv *.ani ./custom-theme/
> cd custom-theme
> ```

```bash
ani2xcur convert Default.ani
```

## Benchmarks

Benchmarked using [hyperfine] against similar projects on GitHub solving the
same problem.

Example command used:

```bash
hyperfine \
    --warmup 15 \
    --setup 'mkdir --parents ./build' \
    --prepare 'rm -rf ./Cursor.toml build/*' \
    '[See "Command used" in following table]' \
    'ani2xcur install --default-init'
```

| Project       | Version | Time               | Command used                      |
| :------------ | :------ | :----------------- | :-------------------------------- |
| [ani2xcur]    | 0.1.0   | 2.1 ms ± 0.3 ms    | `ani2xcur install --default-init` |
| [ani2xcursor] | 1.5.0   | 60.4 ms ± 1.6 ms   | `ani2xcursor --out ./build .`     |
| [win2xcur]    | 0.2.0   | 620.4 ms ± 14.8 ms | `win2xcur ./*.ani -o ./build`     |

## Roadmap

- [x] Automatically scale cursors to standard sizes.
- [ ] Remove `xcursorgen` dependency.
- [ ] Remove need for `build` directory for the `convert` subcommand.
- [ ] Interactive mode to convert individual cursors with Linux remappings.
- [ ] Graphical User Interface

[ani file format]: https://en.wikipedia.org/wiki/ANI_(file_format)
[ani2xcur]: https://github.com/nicdgonzalez/ani2xcur
[ani2xcursor]: https://github.com/yuzujr/ani2xcursor
[hornet cursor]: https://ko-fi.com/s/2e08ca3a58
[hyperfine]: https://github.com/sharkdp/hyperfine
[releases]: https://github.com/nicdgonzalez/ani-to-xcursor/releases
[win2xcur]: https://github.com/quantum5/win2xcur
[x window system]: https://en.wikipedia.org/wiki/X_Window_System
[xcursor]: https://www.x.org/releases/current/doc/man/man3/Xcursor.3.xhtml
