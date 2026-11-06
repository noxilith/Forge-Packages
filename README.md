# Forge recipes

The package repository for [Forge](https://github.com/noxilith/Forge): what
`forge install <name>` installs from. Forge sets it up by itself on first use.

```
forge install hyprland                            # by name
forge build https://github.com/hyprwm/Hyprland    # by link: uses the recipe here
forge build --ignore-repo <link>                  # ignore recipes, auto-detect
forge sync                                        # fetch newer recipes; lists packages you have that can update
```

Two directories, looked up in this order:

* `recipes/`: written and kept up by hand (the packages below).
* `foundry/`: about 5,200 recipes generated from
  [Void Linux](https://voidlinux.org)'s templates (see `foundry/REPORT.md` for what was
  skipped and why, and `foundry/IMPORTED_FROM` for the Void commit). Regenerate with
  `forge-import void <void-packages> --out foundry`. **Never built**: each recipe's
  header lists what the importer could not carry over. The Void templates are
  BSD-2-Clause licensed; `foundry/LICENSE-void-packages` has their notice.

Recipes are build instructions that run in Forge's sandbox, but they are still
code you are choosing to run. Hand-written recipes pin a release tag (or a
commit); imported ones pin a source archive by SHA-256, so builds are repeatable.

## What is here

* **Foundation libraries** that many systems lack: `abseil-cpp`, `re2`, `glaze`,
  `libzip`, `iniparser`, `sdbus-cpp`, `utf8proc`, `libogg`, `libvorbis`,
  `libcanberra`, `xcb-util-errors`, `pciutils`, `lua`, `scdoc`, `tllist`, `fcft`.
* **The Hyprland family**: `hyprland`, `hyprutils`, `hyprlang`, `hyprcursor`,
  `hyprgraphics`, `aquamarine`, `hyprwayland-scanner`, `hyprland-protocols`,
  `hyprwire`, `hyprtoolkit`, `hypridle`, `hyprlock`, `hyprpaper`, `hyprsunset`,
  `hyprpicker`, `xdg-desktop-portal-hyprland`, `hyprpolkitagent`,
  `hyprland-guiutils`, `hyprland-qt-support`, `hyprland-qtutils`, `hyprsysteminfo`.
* **Wayland tools**: `foot`, `fuzzel`, `wl-clipboard`, `cliphist`, `grim`, `slurp`,
  `mako`, `swaybg`, `swaylock`, `swayidle`, `niri`, `xwayland-satellite`.
* **`github-cli`** (the `gh` command; Go, built with a Go toolchain Forge provides).
* **`elora-notes`**: [EloraNotes](https://github.com/noxilith/EloraNotes), the Markdown notes app and its server (Rust). Never built through Forge.

The recipes marked "generated" are written by `gen-recipes` (run `cargo run --manifest-path
gen-recipes/Cargo.toml` to regenerate `recipes/`); `niri` and `github-cli` are written by hand.

## How well tested is this?

Honestly: not very. Forge itself is tested; the recipes were checked like this on
one machine (x86_64, an elogind-based distribution, no systemd headers):

**Built successfully in my test runs (35):** abseil-cpp, aquamarine, cliphist, fcft,
foot, github-cli, glaze, grim, hyprcursor, hyprgraphics, hypridle,
hyprland-protocols, hyprland-qt-support, hyprland-qtutils, hyprlang, hyprlock,
hyprpicker, hyprsunset, hyprtoolkit, hyprutils, hyprwayland-scanner, hyprwire,
iniparser, libogg, libvorbis, libzip, lua, pciutils, re2, sdbus-cpp, slurp, swaybg,
tllist, utf8proc, wl-clipboard.

**Failed, then fixed, and not rebuilt since (11):** fuzzel (needs `scdoc`, now a
build dependency), hyprland-guiutils, hyprpaper, hyprpolkitagent, hyprsysteminfo
(linking against Abseil; Forge now passes the link-time hint), libcanberra (dangling
helper-script links), mako, swayidle (elogind instead of systemd), swaylock (fish
completions went outside the package), xdg-desktop-portal-hyprland (missing
`hyprtoolkit` dependency), scdoc (new).

**Cannot build on that machine:** xcb-util-errors. It ships no generated
`configure` and that machine's `autoconf` is broken. `hyprland` depends on it for
XWayland support.

**Never built:** `hyprland` itself, `xwayland-satellite`. `niri` built once with an
earlier version of its recipe.

If a build fails for you, please open an issue with the output; that is how the
rest gets fixed.

## Notes on specific recipes

* Programs that start other programs (foot, fuzzel, mako, hypridle, swayidle,
  Hyprland, ...) get only their own `--config` pointed at the package, so an XDG
  redirect doesn't leak into everything they launch. The flags were taken from
  each project's documentation and have not all been checked against the built
  programs.
* `hyprlock` and `swaylock` authenticate through the machine's PAM and need a PAM
  service file (`/etc/pam.d/hyprlock`, `/etc/pam.d/swaylock`); Forge never writes
  outside its own directories.
* `sdbus-cpp`, `mako` and `swayidle` use the machine's sd-bus: libsystemd, or
  elogind where systemd's header isn't installed.
* `hyprland-qt-support` and `hyprland-qtutils` need Qt 6 on the machine.

## License

MIT. See [LICENSE](LICENSE).
