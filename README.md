# Nirius: utility commands for the niri wayland compositor

[![builds.sr.ht status](https://builds.sr.ht/~tsdh/nirius.svg)](https://builds.sr.ht/~tsdh/nirius?)
[![License GPL 3 or later](https://img.shields.io/crates/l/nirius.svg)](https://www.gnu.org/licenses/gpl-3.0.en.html)
[![dependency status](https://deps.rs/repo/sourcehut/~tsdh/nirius/status.svg)](https://deps.rs/repo/sourcehut/~tsdh/nirius)
[![Hits-of-Code](https://hitsofcode.com/sourcehut/~tsdh/nirius?branch=main)](https://hitsofcode.com/sourcehut/~tsdh/nirius/view?branch=main)

Some utility commands for the [niri](https://github.com/YaLTeR/niri/) wayland
compositor.  You have to start the `niriusd` daemon and then issue commands
using the `nirius` utility.  The daemon is best started by adding
`spawn-at-startup "niriusd"` to niri's `config.kdl`.

## <a id="installation">Commands</a>

### Focusing matching windows

The focus commands are shortcuts for quickly focusing (or even spawning) the
apps you need all the time.

- `focus [OPTIONS]`: Focuses a matching window if there is one, otherwise exits
  non-zero.  What windows match is specified using the options `--app-id`
  (`-a`) and `--title` (`-t`), both regular expressions.  If there are multiple
  matching windows, the command cycles through them.
- `focus-or-spawn [OPTIONS] [COMMAND]...`: Same behavior as `focus` except that
  it spawns `COMMAND` instead of exiting non-zero if no matching window exists.

### Moving matching windows to the current workspace

Where the focusing commands switch to matching windows where they are, maybe on
a different workspace on a different output, the below commands move matching
windows from elsewhere to the current workspace.

- `move-to-current-workspace [OPTIONS]`: Moves a matching window (same options
  as `focus`) from some unfocused workspace to the currently focused workspace.
  If the `--focus` flag is given, the moved window also gains focus.  If there
  is no matching window, exits non-zero.
- `move-to-current-workspace-or-spawn [OPTIONS] [COMMAND]`: Same behavior as
  `move-to-current-workspace` except that it spawns the given `COMMAND` if
  there is no matching window.

### Categorizing windows with marks

The below commands allow for annotating windows with different marks (or
labels) and provide means to quickly cycle through all windows having the same
mark.

- `toggle-mark [MARK]`: Marks or unmarks a window with the given or default
  mark (which is `__default__`).  Marked windows can be focused using
  `focus-marked`.
- `focus-marked [MARK]`: Focuses the window marked with `MARK`, or the default
  mark `__default__` if not given.  If there are multiple such windows, cycles
  through all of them.
- `list-marked [MARK]`: Lists all windows marked with `MARK`, or the default
  mark if not given, on stdout.  If the `--all` flag is given, list all windows
  of all marks.

### Follow-mode

Windows in follow-mode follow you when switching from one workspace to another
one.  The primary intended use-case are floating music or video player windows.

- `toggle-follow-mode`: Enables or disables *follow mode* for the currently
  focused window.  When switching to another workspace, all windows in follow
  mode are moved to that workspace.

### The scratchpad

Users coming to niri from i3/sway probably know the "scratchpad" which is a
kind of hidden workspace there where you can send windows to which you need
often but don't want to have next to you all the time, e.g., a terminal which
you just need for executing git commands every now and then.  You a command
which shows one scratchpad window as floating window on top of your tiled
windows and moves it back when invoked again.  The below commands implement the
same for niri.  The difference is that niri has no hidden workspace, so the
scratchpad is actually the bottom-most non-empty workspace.  When you focus
that, nirius will move the scratchpad windows to the workspace below.

- `scratchpad-toggle [--app-id PATTERN] [--no-move]`: Moves the current window
  (or a window matching the app-id pattern) to the scratchpad if it's not a
  scratchpad window already. If it is, removes it from the scratchpad, i.e., it's
  just a normal floating window afterwards. The `--no-move` flag toggles the
  scratchpad state without moving the window. Making a scratchpad window tiled
  again also removes its scratchpad state implicitly.
- `scratchpad-show [--app-id PATTERN]`: Shows a window from the scratchpad. If a
  scratchpad window is already shown, moves it back to the scratchpad. When no
  app-id is specified, shows the most recently focused scratchpad window. When
  an app-id pattern is provided, shows a scratchpad window matching that pattern.

### <a id="installation">Installation</a>

Some distros have packaged nirius so that you can install it using your
distro's package manager.  Alternatively, it's easy to build and install it
yourself using `cargo`.

#### Distro packages

The following GNU/Linux and BSD distros package nirius.  Thanks a lot to the
respective package maintainers!  Refer to the [repology
site](https://repology.org/project/nirius/versions) for details.

[![Packaging status](https://repology.org/badge/vertical-allrepos/nirius.svg)](https://repology.org/project/nirius/versions)

#### Building with cargo

You'll need to install the current stable rust toolchain using the one-liner
shown at the [official rust installation
page](https://www.rust-lang.org/tools/install).

Then you can install nirius like so:
```sh
cargo install nirius
```

For getting updates easily, I recommend the cargo `cargo-update` plugin.
```sh
# Install it once.
cargo install cargo-update

# Then you can update all installed rust binary crates including nirius using:
cargo install-update --all

# If you only want to update nirius, you can do so using:
cargo install-update -- nirius
```

## <a id="questions-and-patches">Questions & Patches</a>

For asking questions, sending feedback, or patches, refer to [my public inbox
(mailinglist)](https://lists.sr.ht/~tsdh/public-inbox).  Please mention the
project you are referring to in the subject, e.g., `nirius` (or other projects
in different repositories).

## <a id="bugs">Bugs</a>

It compiles, therefore there are no bugs.  Oh well, if you still found one or
want to request a feature, you can do so
[here](https://todo.sr.ht/~tsdh/nirius).

## <a id="build-status">Build status</a>

[![builds.sr.ht status](https://builds.sr.ht/~tsdh/nirius.svg)](https://builds.sr.ht/~tsdh/nirius?)

## <a id="license">License</a>

Nirius is licensed under the
[GPLv3](https://www.gnu.org/licenses/gpl-3.0.en.html) (or later).
