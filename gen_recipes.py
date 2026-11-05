#!/usr/bin/env python3
"""Generates recipes/*.toml. Kept in the repository so every recipe has the same
shape and the dependency data lives in one place. Run: python3 gen_recipes.py"""
import os, sys

GH = "https://github.com/"
CB = "https://codeberg.org/"

def q(s):
    # TOML literal string where possible (no escaping needed), else a basic string.
    if "'" not in s and "\n" not in s:
        return "'" + s + "'"
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'

def arr(items, multiline=False):
    if not items:
        return "[]"
    if multiline or sum(len(i) for i in items) > 70:
        return "[\n" + "".join(f"  {q(i)},\n" for i in items) + "]"
    return "[" + ", ".join(q(i) for i in items) + "]"

R = []  # recipes, in dependency order

def recipe(name, repo, system, rev=None, deps=(), build_deps=(), tools=(), pkgconfig=(), args=(), steps=(),
           post=(), targets=(), note="", run=None, subdir=None, args_sh=()):
    R.append(dict(name=name, repo=repo, system=system, rev=rev, deps=list(deps), build_deps=list(build_deps),
                  tools=list(tools), pkgconfig=list(pkgconfig), args=list(args), steps=list(steps), post=list(post),
                  targets=list(targets), note=note, run=run, subdir=subdir, args_sh=list(args_sh)))

def render(r):
    o = []
    for l in r["note"].strip().splitlines():
        o.append("# " + l if l else "#")
    o += ["[package]", f"name = {q(r['name'])}", f"repo = {q(r['repo'])}"]
    if r["rev"]:
        o.append(f"rev = {q(r['rev'])}")
    if r["deps"]:
        o.append(f"deps = {arr(r['deps'])}")
    o += ["", "[build]"]
    if r["system"] != "custom":
        o.append(f"system = {q(r['system'])}")
    if r["build_deps"]:
        o.append(f"deps = {arr(r['build_deps'])}   # needed only while building")
    if r["tools"]:
        o.append(f"tools = {arr(r['tools'])}")
    if r["pkgconfig"]:
        o.append("# Host libraries (Forge packages above provide their own).")
        o.append(f"pkgconfig = {arr(r['pkgconfig'])}")
    if r["args"]:
        o.append(f"configure_args = {arr(r['args'])}")
    if r["args_sh"]:
        o.append("# Shell fragments, decided on the build machine.")
        o.append(f"configure_sh = {arr(r['args_sh'], True)}")
    if r["targets"]:
        o.append(f"targets = {arr(r['targets'])}")
    if r["subdir"]:
        o.append(f"subdir = {q(r['subdir'])}")
    if r["steps"]:
        o.append(f"steps = {arr(r['steps'], True)}")
    if r["post"]:
        o += ["", "[install]", f"post = {arr(r['post'], True)}"]
    run = r["run"]
    if run:
        o += ["", "[run]"]
        o.append(f"isolate = {q(run.get('isolate', 'xdg'))}")
        if run.get("dirs"):
            o.append(f"dirs = {arr(run['dirs'])}")
        if run.get("args"):
            o.append(f"args = {arr(run['args'])}")
        if run.get("args_commands"):
            o.append(f"args_commands = {arr(run['args_commands'])}")
        if run.get("env"):
            o += ["", "[run.env]"] + [f"{k} = {q(v)}" for k, v in run["env"].items()]
        if run.get("seed"):
            o += ["", "[run.seed]"] + [f"{q(k)} = {q(v)}" for k, v in run["seed"].items()]
    return "\n".join(o) + "\n"

def spawner(prog, cfgpath, flag="--config", cmd=None):
    """Programs that start other programs must not export XDG redirects to them:
    point only their own --config at the package instead."""
    d = os.path.dirname(cfgpath)
    return dict(isolate="none", dirs=[d], args=[flag, "{data}/" + cfgpath], args_commands=[cmd or prog])

CM = "cmake"
# ---------------------------------------------------------------- foundation libraries
FOUND = "Foundation library that many distributions lack or ship too old."

recipe("abseil-cpp", GH + "abseil/abseil-cpp", CM, rev="20260817.0",
       note=FOUND + "\nBuilt as shared libraries: a shared library that uses a static Abseil leaves unresolved\nsymbols for everything that links it (hyprtoolkit -> hyprpaper).",
       args=["-DCMAKE_CXX_STANDARD=17", "-DABSL_PROPAGATE_CXX_STD=ON", "-DABSL_BUILD_TESTING=OFF",
             "-DCMAKE_POSITION_INDEPENDENT_CODE=ON", "-DBUILD_SHARED_LIBS=ON"])
recipe("re2", GH + "google/re2", CM, deps=["abseil-cpp"],
       note=FOUND + " (needed by Hyprland)", args=["-DRE2_BUILD_TESTING=OFF", "-DBUILD_SHARED_LIBS=ON"])
recipe("glaze", GH + "stephenberry/glaze", CM, rev="v9.1.0",
       note="Header-only JSON/reflection library. Hyprland would otherwise download it while\nconfiguring, which an offline sandboxed build cannot do.",
       args=["-Dglaze_BUILD_EXAMPLES=OFF", "-Dglaze_DEVELOPER_MODE=OFF", "-DBUILD_TESTING=OFF"])
recipe("libzip", GH + "nih-at/libzip", CM, rev="v1.12.1", note=FOUND, pkgconfig=["zlib"], tools=["pkg-config"],
       args=["-DBUILD_DOC=OFF", "-DBUILD_EXAMPLES=OFF", "-DBUILD_REGRESS=OFF", "-DBUILD_TOOLS=OFF",
             "-DENABLE_COMMONCRYPTO=OFF", "-DENABLE_GNUTLS=OFF", "-DENABLE_MBEDTLS=OFF", "-DENABLE_OPENSSL=OFF",
             "-DBUILD_SHARED_LIBS=ON"])
recipe("iniparser", GH + "ndevilla/iniparser", CM, rev="v4.3.2", note=FOUND + " (needed by hyprtoolkit)",
       args=["-DBUILD_SHARED_LIBS=ON", "-DBUILD_EXAMPLES=OFF", "-DBUILD_DOCS=OFF"])
recipe("sdbus-cpp", GH + "Kistler-Group/sdbus-cpp", "custom", rev="v2.3.1",
       note=FOUND + "\nNeeds an sd-bus implementation from the machine: libsystemd, or elogind on systems without\nsystemd. Some systems ship a libsystemd.pc without its headers, so the choice is made by whether\nthe header is really there.",
       steps=['L=default; [ -e /usr/include/systemd/sd-bus.h ] || L=elogind; cmake -S . -B "$FORGE_OBJ" -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$FORGE_PREFIX" -DCMAKE_INSTALL_LIBDIR=lib -DCMAKE_INSTALL_RPATH="$FORGE_PREFIX/lib" -DSDBUSCPP_BUILD_DOCS=OFF -DSDBUSCPP_BUILD_TESTS=OFF -DSDBUSCPP_SDBUS_LIB=$L',
              'cmake --build "$FORGE_OBJ" -j "$FORGE_JOBS"',
              'DESTDIR="$FORGE_STAGE" cmake --install "$FORGE_OBJ"'])
recipe("utf8proc", GH + "JuliaStrings/utf8proc", CM, rev="v2.12.0", note=FOUND, args=["-DBUILD_SHARED_LIBS=ON"])
recipe("libogg", GH + "xiph/ogg", CM, rev="v1.3.6", note=FOUND, args=["-DBUILD_SHARED_LIBS=ON"])
recipe("libvorbis", GH + "xiph/vorbis", CM, rev="v1.3.7", note=FOUND, deps=["libogg"], args=["-DBUILD_SHARED_LIBS=ON"])
recipe("libcanberra", GH + "Distrotech/libcanberra", "custom", rev="edbf1733563f37c650d99fae1e826bf4a46ac3f7", deps=["libvorbis"],
       note=FOUND + " (needed by Hyprland)\nSource: a GitHub mirror of the original, unmaintained upstream, pinned to the commit that ships a\ngenerated `configure`. Its build-aux helper scripts are symlinks to a path that only exists on the\nauthor's machine; any that dangle are replaced from libtool's copies. GTK/PulseAudio/ALSA backends\nare turned off, as Hyprland only needs the library.",
       tools=["pkg-config"],
       steps=['for f in compile config.guess config.sub depcomp install-sh ltmain.sh missing; do [ -e build-aux/$f ] || { rm -f build-aux/$f; for d in /usr/share/libtool/build-aux /usr/share/automake-*; do [ -f $d/$f ] && cp $d/$f build-aux/$f && break; done; }; done',
              './configure --prefix="$FORGE_PREFIX" --disable-gtk --disable-gtk3 --disable-alsa --disable-pulse --disable-oss --disable-gstreamer --disable-tdb --disable-lynx --disable-gtk-doc --disable-gtk-doc-html',
              'make -j"$FORGE_JOBS"',
              'make install DESTDIR="$FORGE_STAGE"'])
recipe("xcb-util-errors", "https://gitlab.freedesktop.org/xorg/lib/libxcb-errors.git", "autotools",
       rev="b90bcfbb7a41716a40fe7fb11518cc89190dbe85",
       note=FOUND + " (libxcb-errors, needed by Hyprland for XWayland)\nPinned to a master commit: the 1.0.1 tag points a submodule at a git:// URL that no longer answers.\nNeeds a working autoconf/automake/libtool on the machine (it ships no generated `configure`).",
       tools=["autoreconf", "python3", "pkg-config"], pkgconfig=["xcb"])
recipe("pciutils", GH + "pciutils/pciutils", "custom", rev="v3.15.0", note=FOUND + " (libpci, needed by hyprsysteminfo)",
       steps=['make -j"$FORGE_JOBS" CC=gcc SHARED=yes ZLIB=no DNS=no PREFIX="$FORGE_PREFIX"',
              'make install install-lib CC=gcc SHARED=yes ZLIB=no DNS=no PREFIX="$FORGE_PREFIX" DESTDIR="$FORGE_STAGE" LDCONFIG=true'])
recipe("lua", GH + "lua/lua", "custom", rev="v5.5.1",
       note="Lua 5.5, which current Hyprland requires. Static, position-independent, with a lua.pc\npkg-config file (the upstream sources ship none).",
       steps=[
           'make -j"$FORGE_JOBS" all CC=gcc MYCFLAGS="-std=c99 -DLUA_USE_LINUX -fPIC"',
           'D="$FORGE_STAGE$FORGE_PREFIX"; mkdir -p "$D/bin" "$D/lib/pkgconfig" "$D/include"; cp lua "$D/bin/"; cp liblua.a "$D/lib/"; cp lua.h luaconf.h lualib.h lauxlib.h "$D/include/"; printf \'extern "C" {\\n#include "lua.h"\\n#include "lualib.h"\\n#include "lauxlib.h"\\n}\\n\' > "$D/include/lua.hpp"; for n in lua lua5.5 lua55; do printf \'prefix=%s\\nName: Lua\\nDescription: Lua language\\nVersion: 5.5.1\\nLibs: -L${prefix}/lib -llua -lm -ldl\\nCflags: -I${prefix}/include\\n\' "$FORGE_PREFIX" > "$D/lib/pkgconfig/$n.pc"; done',
       ])

# ---------------------------------------------------------------- the Hyprland family
HYPR = GH + "hyprwm/"
SCANNER = ["hyprwayland-scanner"]
recipe("hyprwayland-scanner", HYPR + "hyprwayland-scanner", CM, rev="v0.4.6", note="Code generator used while building other Hypr* projects.",
       tools=["pkg-config"], pkgconfig=["pugixml"])
recipe("hyprutils", HYPR + "hyprutils", CM, rev="v0.14.3", tools=["pkg-config"], pkgconfig=["pixman-1"])
recipe("hyprlang", HYPR + "hyprlang", CM, rev="v0.6.8", deps=["hyprutils"])
recipe("hyprwire", HYPR + "hyprwire", CM, rev="v0.3.1", deps=["hyprutils"], tools=["pkg-config"], pkgconfig=["libffi"])
recipe("hyprland-protocols", HYPR + "hyprland-protocols", CM, rev="v0.8.0", note="Wayland protocol definitions used by the Hypr* tools.")
recipe("hyprgraphics", HYPR + "hyprgraphics", CM, rev="v0.6.0", deps=["hyprutils"], tools=["pkg-config"],
       pkgconfig=["pixman-1", "cairo", "libjpeg", "libwebp", "libpng"])
recipe("hyprcursor", HYPR + "hyprcursor", CM, rev="v0.1.13", deps=["hyprlang", "libzip"], tools=["pkg-config"],
       pkgconfig=["librsvg-2.0", "cairo", "tomlplusplus"])
recipe("aquamarine", HYPR + "aquamarine", CM, rev="v0.16.0", deps=["hyprutils"], build_deps=SCANNER, tools=["pkg-config"],
       pkgconfig=["libdrm", "gbm", "libinput", "libseat", "libudev", "pixman-1", "wayland-client", "wayland-protocols", "hwdata", "egl", "glesv2"])
recipe("hyprtoolkit", HYPR + "hyprtoolkit", CM, rev="v0.6.0",
       deps=["hyprutils", "hyprlang", "hyprgraphics", "aquamarine", "iniparser", "abseil-cpp"], build_deps=SCANNER,
       tools=["pkg-config"], pkgconfig=["wayland-client", "wayland-protocols", "egl", "pixman-1", "libdrm", "gbm", "xkbcommon", "pango", "cairo", "pangocairo"])
recipe("hyprland", HYPR + "Hyprland", CM, rev="v0.57.1",
       note="The Hyprland compositor. Everything it needs that distributions often lack is a Forge dependency\n(re2, glaze, lua, libcanberra, libxcb-errors, the Hypr* libraries). The machine must provide the usual\nWayland/graphics libraries below, glslang and a C++ compiler.\nTo log in to it: `forge integrate hyprland`.",
       deps=["hyprutils", "hyprlang", "hyprcursor", "hyprgraphics", "aquamarine", "hyprland-protocols", "re2", "glaze",
             "lua", "libcanberra", "xcb-util-errors"], build_deps=SCANNER, tools=["pkg-config", "python3"],
       pkgconfig=["xkbcommon", "uuid", "wayland-server", "wayland-protocols", "cairo", "pango", "pangocairo", "pixman-1", "xcursor",
                  "libdrm", "libinput", "libeis-1.0", "gbm", "gio-2.0", "muparser", "lcms2", "xcb", "xcb-render", "xcb-xfixes",
                  "xcb-icccm", "xcb-composite", "xcb-res", "egl", "glesv2"],
       run=dict(isolate="none", dirs=["config/hypr"], args=["--config", "{data}/config/hypr/hyprland.conf"], args_commands=["Hyprland"]))
recipe("hypridle", HYPR + "hypridle", CM, rev="v0.1.8", deps=["hyprlang", "hyprutils", "hyprland-protocols", "sdbus-cpp"], build_deps=SCANNER,
       tools=["pkg-config"], pkgconfig=["wayland-client", "wayland-protocols"], run=spawner("hypridle", "config/hypr/hypridle.conf"))
recipe("hyprlock", HYPR + "hyprlock", CM, rev="v0.9.7", deps=["hyprlang", "hyprutils", "hyprgraphics", "hyprland-protocols", "sdbus-cpp"],
       build_deps=SCANNER,
       note="Screen locker. Authenticates through the machine's PAM, so the system needs a PAM service\nfile for it (`/etc/pam.d/hyprlock`); Forge never writes outside its own directories.",
       tools=["pkg-config"], pkgconfig=["wayland-client", "wayland-protocols", "wayland-egl", "egl", "xkbcommon", "cairo", "pangocairo", "libdrm", "gbm", "pam"])
recipe("hyprpaper", HYPR + "hyprpaper", CM, rev="v0.8.5", deps=["hyprlang", "hyprutils", "hyprtoolkit", "hyprwire"], build_deps=SCANNER,
       tools=["pkg-config"], pkgconfig=["wayland-client", "pixman-1", "libdrm"])
recipe("hyprsunset", HYPR + "hyprsunset", CM, rev="v0.4.0", deps=["hyprutils", "hyprlang", "hyprland-protocols"], build_deps=SCANNER,
       tools=["pkg-config"], pkgconfig=["wayland-client", "wayland-protocols"])
recipe("hyprpicker", HYPR + "hyprpicker", CM, rev="v0.4.8", deps=["hyprutils"], build_deps=SCANNER, tools=["pkg-config"],
       pkgconfig=["wayland-client", "wayland-protocols", "xkbcommon", "cairo", "pango", "pangocairo", "libjpeg"])
recipe("xdg-desktop-portal-hyprland", HYPR + "xdg-desktop-portal-hyprland", CM, rev="v1.5.0",
       deps=["hyprutils", "hyprlang", "hyprland-protocols", "sdbus-cpp", "hyprtoolkit"], build_deps=SCANNER,
       note="Screen sharing and screenshots for sandboxed apps. The repository also contains a Meson file for\nits systemd unit, so the build system is named explicitly.",
       tools=["pkg-config"], pkgconfig=["wayland-client", "wayland-protocols", "libpipewire-0.3", "libspa-0.2", "libdrm", "uuid", "gbm"])
recipe("hyprpolkitagent", HYPR + "hyprpolkitagent", CM, rev="v0.2.0", deps=["hyprutils", "hyprtoolkit", "hyprgraphics", "hyprlang", "sdbus-cpp"],
       tools=["pkg-config"], pkgconfig=["pixman-1", "libdrm"])
recipe("hyprland-guiutils", HYPR + "hyprland-guiutils", CM, rev="v0.2.2", deps=["hyprlang", "hyprutils", "hyprtoolkit"],
       tools=["pkg-config"], pkgconfig=["pixman-1", "xkbcommon", "libdrm"])
recipe("hyprland-qt-support", HYPR + "hyprland-qt-support", CM, rev="v0.1.0", deps=["hyprlang"],
       note="Needs Qt 6 on the machine. Its QML module is installed inside the package (by default it asks\nfor Qt's own system directory).", tools=["pkg-config"],
       args=["-DINSTALL_QML_PREFIX=lib/qt6/qml"])
recipe("hyprland-qtutils", HYPR + "hyprland-qtutils", CM, rev="v0.1.5", deps=["hyprutils", "hyprland-qt-support"],
       note="Needs Qt 6 on the machine. Finds hyprland-qt-support's QML module through QML_IMPORT_PATH.", tools=["pkg-config"],
       run=dict(isolate="none", env={"QML_IMPORT_PATH": "{dep:hyprland-qt-support}/lib/qt6/qml"}))
recipe("hyprsysteminfo", HYPR + "hyprsysteminfo", CM, rev="v0.2.0", deps=["hyprtoolkit", "hyprutils", "glaze", "pciutils"],
       tools=["pkg-config"], pkgconfig=["libdrm", "pixman-1"])

# ---------------------------------------------------------------- Wayland desktop tools (Meson, mostly)
MESON_MAN = ["-Dman-pages=disabled"]
SD = '$([ -e /usr/include/systemd/sd-bus.h ] && echo %s || echo %s)'   # sd-bus provider: systemd, else elogind
NOFISH = ["-Dfish-completions=false"]   # Meson installs these to an absolute system path
SDBUS_NOTE = "Uses the machine's sd-bus: libsystemd, or elogind where systemd's header isn't installed."
recipe("scdoc", "https://git.sr.ht/~sircmpwn/scdoc", "custom", rev="1.11.5",
       note="Man-page compiler. A build-time tool for projects (fuzzel) whose build insists on their documentation.",
       steps=['make -j"$FORGE_JOBS" CC=gcc PREFIX="$FORGE_PREFIX"', 'make install CC=gcc PREFIX="$FORGE_PREFIX" DESTDIR="$FORGE_STAGE"'])
recipe("tllist", CB + "dnkl/tllist", "meson", rev="1.1.0", note="Header-only list library used by foot, fcft and fuzzel.")
recipe("fcft", CB + "dnkl/fcft", "meson", rev="3.3.3", deps=["tllist", "utf8proc"], note="Font loading and glyph rasterization library for foot and fuzzel.",
       tools=["pkg-config"], pkgconfig=["freetype2", "harfbuzz", "fontconfig", "pixman-1"])
recipe("foot", CB + "dnkl/foot", "meson", rev="1.28.0", deps=["fcft", "tllist", "utf8proc"],
       note="Wayland terminal. It starts your shell, so only its own --config is pointed at the package\n(an XDG redirect would leak into every program you run in it).",
       tools=["pkg-config", "wayland-scanner"], pkgconfig=["wayland-client", "wayland-cursor", "wayland-protocols", "xkbcommon", "fontconfig", "pixman-1"],
       args=["-Ddocs=disabled"], run=spawner("foot", "config/foot/foot.ini", "--config"))
recipe("fuzzel", CB + "dnkl/fuzzel", "meson", rev="1.15.0", deps=["fcft", "tllist"], build_deps=["scdoc"],
       note="Application launcher. It starts programs, so only its own --config is pointed at the package.",
       tools=["pkg-config", "wayland-scanner"], pkgconfig=["wayland-client", "wayland-cursor", "wayland-protocols", "xkbcommon", "fontconfig", "pixman-1", "cairo"],
       run=spawner("fuzzel", "config/fuzzel/fuzzel.ini", "--config"))
recipe("wl-clipboard", GH + "bugaevc/wl-clipboard", "meson", rev="v2.3.0", note="wl-copy and wl-paste: clipboard access from the command line (the tools gh looks for).",
       tools=["pkg-config", "wayland-scanner"], pkgconfig=["wayland-client", "wayland-protocols"],
       args=["-Dzshcompletiondir=share/zsh/site-functions", "-Dfishcompletiondir=share/fish/vendor_completions.d"])
recipe("cliphist", GH + "sentriz/cliphist", "go", rev="v0.7.0", deps=["wl-clipboard"], note="Clipboard history. Uses wl-clipboard at runtime.", targets=["."])
recipe("grim", GH + "emersion/grim", "meson", rev="v1.4.0", note="Screenshots.", tools=["pkg-config", "wayland-scanner"],
       pkgconfig=["libpng", "libjpeg", "pixman-1", "wayland-client", "wayland-protocols"], args=MESON_MAN + NOFISH)
recipe("slurp", GH + "emersion/slurp", "meson", rev="v1.5.0", note="Select a region of the screen (pairs with grim).", tools=["pkg-config", "wayland-scanner"],
       pkgconfig=["cairo", "wayland-client", "wayland-cursor", "wayland-protocols", "xkbcommon"], args=MESON_MAN)
recipe("mako", GH + "emersion/mako", "meson", rev="v1.11.0", note="Notification daemon. Starts programs on actions, so only --config is pointed at the package.\n" + SDBUS_NOTE,
       tools=["pkg-config", "wayland-scanner"], pkgconfig=["cairo", "pango", "pangocairo", "glib-2.0", "gobject-2.0", "wayland-client", "wayland-protocols", "wayland-cursor", "gdk-pixbuf-2.0"],
       args=MESON_MAN, args_sh=["-Dsd-bus-provider=" + SD % ("auto", "libelogind")], run=spawner("mako", "config/mako/config", "--config"))
recipe("swaybg", GH + "swaywm/swaybg", "meson", rev="v1.2.2", note="Wallpaper.", tools=["pkg-config", "wayland-scanner"],
       pkgconfig=["wayland-client", "wayland-protocols", "cairo", "gdk-pixbuf-2.0"], args=MESON_MAN)
recipe("swaylock", GH + "swaywm/swaylock", "meson", rev="v1.8.6", note="Screen locker. Uses the machine's PAM and needs a PAM service file for `swaylock`.",
       tools=["pkg-config", "wayland-scanner"], pkgconfig=["wayland-client", "wayland-protocols", "xkbcommon", "cairo", "gdk-pixbuf-2.0"], args=MESON_MAN + NOFISH,
       run=spawner("swaylock", "config/swaylock/config", "--config"))
recipe("swayidle", GH + "swaywm/swayidle", "meson", rev="v1.9.1", note="Idle manager. Runs commands, so only its own config is pointed at the package.\n" + SDBUS_NOTE,
       tools=["pkg-config", "wayland-scanner"], pkgconfig=["wayland-client", "wayland-protocols", "wayland-server"], args=MESON_MAN + NOFISH,
       args_sh=["-Dlogind-provider=" + SD % ("systemd", "elogind")], run=spawner("swayidle", "config/swayidle/config", "-C"))
recipe("xwayland-satellite", GH + "Supreeeme/xwayland-satellite", "cargo", rev="v0.8",
       note="Runs X11 programs under compositors without built-in XWayland (such as niri).\nNeeds the `Xwayland` binary on the machine.",
       tools=["pkg-config"], pkgconfig=["xcb", "xcb-cursor"])

if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(here, "recipes")
    # Hand-written recipes that this script does not own.
    keep = {"niri", "github-cli"}
    for r in R:
        assert r["name"] not in keep, r["name"]
        with open(os.path.join(out, r["name"] + ".toml"), "w") as f:
            f.write(render(r))
    print(f"wrote {len(R)} recipes; hand-written kept: {sorted(keep)}")
    if "--names" in sys.argv:
        print(" ".join(r["name"] for r in R))
