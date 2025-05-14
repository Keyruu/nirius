# Nirius: utility commands for niri

[![builds.sr.ht status](https://builds.sr.ht/~tsdh/nirius.svg)](https://builds.sr.ht/~tsdh/nirius?)
[![License GPL 3 or later](https://img.shields.io/crates/l/nirius.svg)](https://www.gnu.org/licenses/gpl-3.0.en.html)
[![dependency status](https://deps.rs/repo/sourcehut/~tsdh/nirius/status.svg)](https://deps.rs/repo/sourcehut/~tsdh/nirius)
[![Hits-of-Code](https://hitsofcode.com/sourcehut/~tsdh/nirius?branch=main)](https://hitsofcode.com/sourcehut/~tsdh/nirius/view?branch=main)

Some utility commands for the [niri](https://github.com/YaLTeR/niri/) wayland
compositor.  You have to start the `niriusd` daemon and then issue commands
using the `nirius` utility.  The daemon is best started by adding
`spawn-at-startup "niriusd"` to niri's `config.kdl`.

## Commands

- `focus [OPTIONS]`: Focuses a matching window if there is one, otherwise exits
  non-zero.  What windows match is specified using the options `--app-id`
  (`-a`) and `--title` (`-t`), both regular expressions.  If there are multiple
  matching windows, the command cycles through them.
- `focus-or-spawn [OPTIONS] [COMMAND]...`: Same behavior as `focus` except that
  it spawns `COMMAND` instead of exiting non-zero if no matching window exists.
- `nop`: Does nothing except having the side-effect of clearing the list of
  already visited windows captured during a sequence of `focus` or
  `focus-or-spawn` commands.

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
