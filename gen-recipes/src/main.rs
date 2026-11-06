//! Generates `recipes/*.toml`. Kept in the repository so every recipe has the same shape and
//! the dependency data lives in one place. Run from the repository root:
//!
//!     cargo run --manifest-path gen-recipes/Cargo.toml [-- --names]

use std::fs;
use std::path::Path;

const GH: &str = "https://github.com/";
const CB: &str = "https://codeberg.org/";

/// A TOML literal string where possible (no escaping needed), else a basic string.
fn q(s: &str) -> String {
    if !s.contains('\'') && !s.contains('\n') {
        format!("'{s}'")
    } else {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn arr(items: &[String], multiline: bool) -> String {
    if items.is_empty() {
        return "[]".into();
    }
    if multiline || items.iter().map(|i| i.chars().count()).sum::<usize>() > 70 {
        let body: String = items.iter().map(|i| format!("  {},\n", q(i))).collect();
        format!("[\n{body}]")
    } else {
        format!("[{}]", items.iter().map(|i| q(i)).collect::<Vec<_>>().join(", "))
    }
}

fn v(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[derive(Default, Clone)]
struct Run {
    isolate: Option<String>,
    dirs: Vec<String>,
    args: Vec<String>,
    args_commands: Vec<String>,
    env: Vec<(String, String)>,
}

#[derive(Default)]
struct Recipe {
    name: String,
    repo: String,
    system: String,
    rev: Option<String>,
    deps: Vec<String>,
    build_deps: Vec<String>,
    tools: Vec<String>,
    pkgconfig: Vec<String>,
    args: Vec<String>,
    args_sh: Vec<String>,
    steps: Vec<String>,
    post: Vec<String>,
    targets: Vec<String>,
    note: String,
    run: Option<Run>,
}

fn recipe(name: &str, repo: &str, system: &str) -> Recipe {
    Recipe { name: name.into(), repo: repo.into(), system: system.into(), ..Default::default() }
}

impl Recipe {
    fn rev(mut self, r: &str) -> Self {
        self.rev = Some(r.into());
        self
    }
    fn deps(mut self, d: &[&str]) -> Self {
        self.deps = v(d);
        self
    }
    fn build_deps(mut self, d: &[&str]) -> Self {
        self.build_deps = v(d);
        self
    }
    fn tools(mut self, d: &[&str]) -> Self {
        self.tools = v(d);
        self
    }
    fn pkgconfig(mut self, d: &[&str]) -> Self {
        self.pkgconfig = v(d);
        self
    }
    fn args(mut self, d: &[&str]) -> Self {
        self.args = v(d);
        self
    }
    fn args_sh(mut self, d: Vec<String>) -> Self {
        self.args_sh = d;
        self
    }
    fn steps(mut self, d: &[&str]) -> Self {
        self.steps = v(d);
        self
    }
    fn post(mut self, d: &[&str]) -> Self {
        self.post = v(d);
        self
    }
    fn targets(mut self, d: &[&str]) -> Self {
        self.targets = v(d);
        self
    }
    fn note(mut self, n: &str) -> Self {
        self.note = n.into();
        self
    }
    fn run(mut self, r: Run) -> Self {
        self.run = Some(r);
        self
    }

    fn render(&self) -> String {
        let mut o: Vec<String> = Vec::new();
        for l in self.note.trim().lines() {
            o.push(if l.is_empty() { "#".into() } else { format!("# {l}") });
        }
        o.push("[package]".into());
        o.push(format!("name = {}", q(&self.name)));
        o.push(format!("repo = {}", q(&self.repo)));
        if let Some(r) = &self.rev {
            o.push(format!("rev = {}", q(r)));
        }
        if !self.deps.is_empty() {
            o.push(format!("deps = {}", arr(&self.deps, false)));
        }
        o.push(String::new());
        o.push("[build]".into());
        if self.system != "custom" {
            o.push(format!("system = {}", q(&self.system)));
        }
        if !self.build_deps.is_empty() {
            o.push(format!("deps = {}   # needed only while building", arr(&self.build_deps, false)));
        }
        if !self.tools.is_empty() {
            o.push(format!("tools = {}", arr(&self.tools, false)));
        }
        if !self.pkgconfig.is_empty() {
            o.push("# Host libraries (Forge packages above provide their own).".into());
            o.push(format!("pkgconfig = {}", arr(&self.pkgconfig, false)));
        }
        if !self.args.is_empty() {
            o.push(format!("configure_args = {}", arr(&self.args, false)));
        }
        if !self.args_sh.is_empty() {
            o.push("# Shell fragments, decided on the build machine.".into());
            o.push(format!("configure_sh = {}", arr(&self.args_sh, true)));
        }
        if !self.targets.is_empty() {
            o.push(format!("targets = {}", arr(&self.targets, false)));
        }
        if !self.steps.is_empty() {
            o.push(format!("steps = {}", arr(&self.steps, true)));
        }
        if !self.post.is_empty() {
            o.push(String::new());
            o.push("[install]".into());
            o.push(format!("post = {}", arr(&self.post, true)));
        }
        if let Some(run) = &self.run {
            o.push(String::new());
            o.push("[run]".into());
            o.push(format!("isolate = {}", q(run.isolate.as_deref().unwrap_or("xdg"))));
            if !run.dirs.is_empty() {
                o.push(format!("dirs = {}", arr(&run.dirs, false)));
            }
            if !run.args.is_empty() {
                o.push(format!("args = {}", arr(&run.args, false)));
            }
            if !run.args_commands.is_empty() {
                o.push(format!("args_commands = {}", arr(&run.args_commands, false)));
            }
            if !run.env.is_empty() {
                o.push(String::new());
                o.push("[run.env]".into());
                o.extend(run.env.iter().map(|(k, val)| format!("{k} = {}", q(val))));
            }
        }
        o.join("\n") + "\n"
    }
}

/// Programs that start other programs must not export XDG redirects to them:
/// point only their own --config at the package instead.
fn spawner(prog: &str, cfgpath: &str, flag: &str) -> Run {
    let dir = cfgpath.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    Run {
        isolate: Some("none".into()),
        dirs: v(&[dir]),
        args: vec![flag.into(), format!("{{data}}/{cfgpath}")],
        args_commands: v(&[prog]),
        ..Default::default()
    }
}

/// sd-bus provider: systemd, else elogind.
fn sd(a: &str, b: &str) -> String {
    format!("$([ -e /usr/include/systemd/sd-bus.h ] && echo {a} || echo {b})")
}

fn all() -> Vec<Recipe> {
    let cm = "cmake";
    let found = "Foundation library that many distributions lack or ship too old.";
    let h = |n: &str| format!("{GH}hyprwm/{n}");
    let scanner = ["hyprwayland-scanner"];
    let meson_man = ["-Dman-pages=disabled"];
    let nofish = ["-Dfish-completions=false"]; // Meson installs these to an absolute system path
    let nocomp = ["-Dbash-completions=false", "-Dzsh-completions=false", "-Dfish-completions=false"];
    let sdbus_note = "Uses the machine's sd-bus: libsystemd, or elogind where systemd's header isn't installed.";
    let mut r: Vec<Recipe> = Vec::new();

    // ---------------------------------------------------------------- foundation libraries
    r.push(recipe("abseil-cpp", &format!("{GH}abseil/abseil-cpp"), cm).rev("20260817.0")
        .note(&format!("{found}\nBuilt as shared libraries: a shared library that uses a static Abseil leaves unresolved\nsymbols for everything that links it (hyprtoolkit -> hyprpaper)."))
        .args(&["-DCMAKE_CXX_STANDARD=17", "-DABSL_PROPAGATE_CXX_STD=ON", "-DABSL_BUILD_TESTING=OFF", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON", "-DBUILD_SHARED_LIBS=ON"]));
    r.push(recipe("re2", &format!("{GH}google/re2"), cm).deps(&["abseil-cpp"])
        .note(&format!("{found} (needed by Hyprland)")).args(&["-DRE2_BUILD_TESTING=OFF", "-DBUILD_SHARED_LIBS=ON"]));
    r.push(recipe("glaze", &format!("{GH}stephenberry/glaze"), cm).rev("v9.1.0")
        .note("Header-only JSON/reflection library. Hyprland would otherwise download it while\nconfiguring, which an offline sandboxed build cannot do.")
        .args(&["-Dglaze_BUILD_EXAMPLES=OFF", "-Dglaze_DEVELOPER_MODE=OFF", "-DBUILD_TESTING=OFF"]));
    r.push(recipe("libzip", &format!("{GH}nih-at/libzip"), cm).rev("v1.12.1").note(found).pkgconfig(&["zlib"]).tools(&["pkg-config"])
        .args(&["-DBUILD_DOC=OFF", "-DBUILD_EXAMPLES=OFF", "-DBUILD_REGRESS=OFF", "-DBUILD_TOOLS=OFF", "-DENABLE_COMMONCRYPTO=OFF", "-DENABLE_GNUTLS=OFF", "-DENABLE_MBEDTLS=OFF", "-DENABLE_OPENSSL=OFF", "-DBUILD_SHARED_LIBS=ON"]));
    r.push(recipe("iniparser", &format!("{GH}ndevilla/iniparser"), cm).rev("v4.3.2").note(&format!("{found} (needed by hyprtoolkit)"))
        .args(&["-DBUILD_SHARED_LIBS=ON", "-DBUILD_EXAMPLES=OFF", "-DBUILD_DOCS=OFF"]));
    r.push(recipe("sdbus-cpp", &format!("{GH}Kistler-Group/sdbus-cpp"), "custom").rev("v2.3.1")
        .note(&format!("{found}\nNeeds an sd-bus implementation from the machine: libsystemd, or elogind on systems without\nsystemd. Some systems ship a libsystemd.pc without its headers, so the choice is made by whether\nthe header is really there."))
        .steps(&[
            r#"L=default; [ -e /usr/include/systemd/sd-bus.h ] || L=elogind; cmake -S . -B "$FORGE_OBJ" -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$FORGE_PREFIX" -DCMAKE_INSTALL_LIBDIR=lib -DCMAKE_INSTALL_RPATH="$FORGE_PREFIX/lib" -DSDBUSCPP_BUILD_DOCS=OFF -DSDBUSCPP_BUILD_TESTS=OFF -DSDBUSCPP_SDBUS_LIB=$L"#,
            r#"cmake --build "$FORGE_OBJ" -j "$FORGE_JOBS""#,
            r#"DESTDIR="$FORGE_STAGE" cmake --install "$FORGE_OBJ""#,
        ]));
    r.push(recipe("utf8proc", &format!("{GH}JuliaStrings/utf8proc"), cm).rev("v2.12.0").note(found).args(&["-DBUILD_SHARED_LIBS=ON"]));
    r.push(recipe("libogg", &format!("{GH}xiph/ogg"), cm).rev("v1.3.6").note(found).args(&["-DBUILD_SHARED_LIBS=ON"]));
    r.push(recipe("libvorbis", &format!("{GH}xiph/vorbis"), cm).rev("v1.3.7").note(found).deps(&["libogg"]).args(&["-DBUILD_SHARED_LIBS=ON", "-DCMAKE_POLICY_VERSION_MINIMUM=3.5"]));
    r.push(recipe("libcanberra", &format!("{GH}Distrotech/libcanberra"), "custom").rev("edbf1733563f37c650d99fae1e826bf4a46ac3f7").deps(&["libvorbis"])
        .note(&format!("{found} (needed by Hyprland)\nSource: a GitHub mirror of the original, unmaintained upstream, pinned to the commit that ships a\ngenerated `configure`. Its build-aux helper scripts are symlinks to a path that only exists on the\nauthor's machine; any that dangle are replaced from libtool's copies. GTK/PulseAudio/ALSA backends\nare turned off, as Hyprland only needs the library. Its Makefiles want to re-run one specific automake\nversion; that regeneration is switched off (the shipped files are used as they are)."))
        .tools(&["pkg-config"])
        .steps(&[
            r#"for f in compile config.guess config.sub depcomp install-sh ltmain.sh missing; do [ -e build-aux/$f ] || { rm -f build-aux/$f; for d in /usr/share/libtool/build-aux /usr/share/automake-*; do [ -f $d/$f ] && cp $d/$f build-aux/$f && break; done; }; done"#,
            r#"./configure --prefix="$FORGE_PREFIX" --disable-gtk --disable-gtk3 --disable-alsa --disable-pulse --disable-oss --disable-gstreamer --disable-tdb --disable-lynx --disable-gtk-doc --disable-gtk-doc-html"#,
            r#"make -j"$FORGE_JOBS" ACLOCAL=: AUTOMAKE=: AUTOCONF=: AUTOHEADER=:"#,
            r#"make install DESTDIR="$FORGE_STAGE" ACLOCAL=: AUTOMAKE=: AUTOCONF=: AUTOHEADER=:"#,
        ]));
    r.push(recipe("xcb-util-errors", "https://gitlab.freedesktop.org/xorg/lib/libxcb-errors.git", "autotools").rev("b90bcfbb7a41716a40fe7fb11518cc89190dbe85")
        .note(&format!("{found} (libxcb-errors, needed by Hyprland for XWayland)\nPinned to a master commit: the 1.0.1 tag points a submodule at a git:// URL that no longer answers.\nNeeds a working autoconf/automake/libtool on the machine (it ships no generated `configure`)."))
        .tools(&["autoreconf", "python3", "pkg-config"]).pkgconfig(&["xcb"]));
    r.push(recipe("pciutils", &format!("{GH}pciutils/pciutils"), "custom").rev("v3.15.0").note(&format!("{found} (libpci, needed by hyprsysteminfo)"))
        .steps(&[
            r#"make -j"$FORGE_JOBS" CC=gcc SHARED=yes ZLIB=no DNS=no PREFIX="$FORGE_PREFIX""#,
            r#"make install install-lib CC=gcc SHARED=yes ZLIB=no DNS=no PREFIX="$FORGE_PREFIX" DESTDIR="$FORGE_STAGE" LDCONFIG=true"#,
        ]));
    r.push(recipe("lua", &format!("{GH}lua/lua"), "custom").rev("v5.5.1")
        .note("Lua 5.5, which current Hyprland requires. Static, position-independent, with a lua.pc\npkg-config file (the upstream sources ship none).")
        .steps(&[
            r#"make -j"$FORGE_JOBS" all CC=gcc MYCFLAGS="-std=c99 -DLUA_USE_LINUX -fPIC""#,
            r#"D="$FORGE_STAGE$FORGE_PREFIX"; mkdir -p "$D/bin" "$D/lib/pkgconfig" "$D/include"; cp lua "$D/bin/"; cp liblua.a "$D/lib/"; cp lua.h luaconf.h lualib.h lauxlib.h "$D/include/"; printf 'extern "C" {\n#include "lua.h"\n#include "lualib.h"\n#include "lauxlib.h"\n}\n' > "$D/include/lua.hpp"; for n in lua lua5.5 lua55; do printf 'prefix=%s\nName: Lua\nDescription: Lua language\nVersion: 5.5.1\nLibs: -L${prefix}/lib -llua -lm -ldl\nCflags: -I${prefix}/include\n' "$FORGE_PREFIX" > "$D/lib/pkgconfig/$n.pc"; done"#,
        ]));

    // ---------------------------------------------------------------- the Hyprland family
    r.push(recipe("hyprwayland-scanner", &h("hyprwayland-scanner"), cm).rev("v0.4.6").note("Code generator used while building other Hypr* projects.").tools(&["pkg-config"]).pkgconfig(&["pugixml"]));
    r.push(recipe("hyprutils", &h("hyprutils"), cm).rev("v0.14.3").tools(&["pkg-config"]).pkgconfig(&["pixman-1"]));
    r.push(recipe("hyprlang", &h("hyprlang"), cm).rev("v0.6.8").deps(&["hyprutils"]));
    r.push(recipe("hyprwire", &h("hyprwire"), cm).rev("v0.3.1").deps(&["hyprutils"]).tools(&["pkg-config"]).pkgconfig(&["libffi"]));
    r.push(recipe("hyprland-protocols", &h("hyprland-protocols"), cm).rev("v0.8.0").note("Wayland protocol definitions used by the Hypr* tools."));
    r.push(recipe("hyprgraphics", &h("hyprgraphics"), cm).rev("v0.6.0").deps(&["hyprutils"]).tools(&["pkg-config"]).pkgconfig(&["pixman-1", "cairo", "libjpeg", "libwebp", "libpng", "libmagic"]));
    r.push(recipe("hyprcursor", &h("hyprcursor"), cm).rev("v0.1.13").deps(&["hyprlang", "libzip"]).tools(&["pkg-config"]).pkgconfig(&["librsvg-2.0", "cairo", "tomlplusplus"]));
    r.push(recipe("aquamarine", &h("aquamarine"), cm).rev("v0.16.0").deps(&["hyprutils"]).build_deps(&scanner).tools(&["pkg-config"])
        .pkgconfig(&["libdrm", "gbm", "libinput", "libseat", "libudev", "pixman-1", "wayland-client", "wayland-protocols", "hwdata", "egl", "glesv2"]));
    r.push(recipe("hyprtoolkit", &h("hyprtoolkit"), cm).rev("v0.6.0")
        .deps(&["hyprutils", "hyprlang", "hyprgraphics", "aquamarine", "iniparser", "abseil-cpp"]).build_deps(&scanner)
        .tools(&["pkg-config"]).pkgconfig(&["wayland-client", "wayland-protocols", "egl", "pixman-1", "libdrm", "gbm", "xkbcommon", "pango", "cairo", "pangocairo"]));
    r.push(recipe("hyprland", &h("Hyprland"), cm).rev("v0.57.1")
        .note("The Hyprland compositor. Everything it needs that distributions often lack is a Forge dependency\n(re2, glaze, lua, libcanberra, libxcb-errors, the Hypr* libraries). The machine must provide the usual\nWayland/graphics libraries below, glslang and a C++ compiler.\nTo log in to it: `forge integrate hyprland`.")
        .deps(&["hyprutils", "hyprlang", "hyprcursor", "hyprgraphics", "aquamarine", "hyprland-protocols", "re2", "glaze", "lua", "libcanberra", "xcb-util-errors"])
        .build_deps(&scanner).tools(&["pkg-config", "python3"])
        .pkgconfig(&["xkbcommon", "uuid", "wayland-server", "wayland-protocols", "cairo", "pango", "pangocairo", "pixman-1", "xcursor", "libdrm", "libinput", "libeis-1.0", "gbm", "gio-2.0", "muparser", "lcms2", "xcb", "xcb-render", "xcb-xfixes", "xcb-icccm", "xcb-composite", "xcb-res", "egl", "glesv2"])
        .run(Run { isolate: Some("none".into()), dirs: v(&["config/hypr"]), args: v(&["--config", "{data}/config/hypr/hyprland.conf"]), args_commands: v(&["Hyprland"]), ..Default::default() }));
    r.push(recipe("hypridle", &h("hypridle"), cm).rev("v0.1.8").deps(&["hyprlang", "hyprutils", "hyprland-protocols", "sdbus-cpp"]).build_deps(&scanner)
        .tools(&["pkg-config"]).pkgconfig(&["wayland-client", "wayland-protocols"]).run(spawner("hypridle", "config/hypr/hypridle.conf", "--config")));
    r.push(recipe("hyprlock", &h("hyprlock"), cm).rev("v0.9.7").deps(&["hyprlang", "hyprutils", "hyprgraphics", "hyprland-protocols", "sdbus-cpp"])
        .build_deps(&scanner)
        .note("Screen locker. Authenticates through the machine's PAM, so the system needs a PAM service\nfile for it (`/etc/pam.d/hyprlock`); Forge never writes outside its own directories. A copy to use\nis shipped in the package, at etc/pam.d/hyprlock.")
        .tools(&["pkg-config"]).pkgconfig(&["wayland-client", "wayland-protocols", "wayland-egl", "egl", "xkbcommon", "cairo", "pangocairo", "libdrm", "gbm", "pam"]).post(&[r#"[ ! -d "$FORGE_STAGE/etc" ] || { mkdir -p "$FORGE_STAGE$FORGE_PREFIX/etc/pam.d" && cp "$(find "$FORGE_STAGE/etc" -name hyprlock -type f | head -1)" "$FORGE_STAGE$FORGE_PREFIX/etc/pam.d/hyprlock" && rm -rf "$FORGE_STAGE/etc"; }"#]));
    r.push(recipe("hyprpaper", &h("hyprpaper"), cm).rev("v0.8.5").deps(&["hyprlang", "hyprutils", "hyprtoolkit", "hyprwire"]).build_deps(&scanner)
        .tools(&["pkg-config"]).pkgconfig(&["wayland-client", "pixman-1", "libdrm"]));
    r.push(recipe("hyprsunset", &h("hyprsunset"), cm).rev("v0.4.0").deps(&["hyprutils", "hyprlang", "hyprland-protocols"]).build_deps(&scanner)
        .note("Where systemd is present the build wants to install its user unit into /usr; it is kept inside the\npackage (lib/systemd/user) instead.")
        .tools(&["pkg-config"]).pkgconfig(&["wayland-client", "wayland-protocols"])
        .post(&[r#"[ ! -d "$FORGE_STAGE/usr/lib/systemd/user" ] || { mkdir -p "$FORGE_STAGE$FORGE_PREFIX/lib/systemd/user" && mv "$FORGE_STAGE"/usr/lib/systemd/user/* "$FORGE_STAGE$FORGE_PREFIX/lib/systemd/user/" && find "$FORGE_STAGE/usr" -type d -empty -delete; }"#]));
    r.push(recipe("hyprpicker", &h("hyprpicker"), cm).rev("v0.4.8").deps(&["hyprutils"]).build_deps(&scanner).tools(&["pkg-config"])
        .pkgconfig(&["wayland-client", "wayland-protocols", "xkbcommon", "cairo", "pango", "pangocairo", "libjpeg"]));
    r.push(recipe("xdg-desktop-portal-hyprland", &h("xdg-desktop-portal-hyprland"), cm).rev("v1.5.0")
        .deps(&["hyprutils", "hyprlang", "hyprland-protocols", "sdbus-cpp", "hyprtoolkit"]).build_deps(&scanner)
        .note("Screen sharing and screenshots for sandboxed apps. The repository also contains a Meson file for\nits systemd unit, so the build system is named explicitly.")
        .tools(&["pkg-config"]).pkgconfig(&["wayland-client", "wayland-protocols", "libpipewire-0.3", "libspa-0.2", "libdrm", "uuid", "gbm"]));
    r.push(recipe("hyprpolkitagent", &h("hyprpolkitagent"), cm).rev("v0.2.0").deps(&["hyprutils", "hyprtoolkit", "hyprgraphics", "hyprlang", "sdbus-cpp"])
        .tools(&["pkg-config"]).pkgconfig(&["pixman-1", "libdrm"]));
    r.push(recipe("hyprland-guiutils", &h("hyprland-guiutils"), cm).rev("v0.2.2").deps(&["hyprlang", "hyprutils", "hyprtoolkit"])
        .tools(&["pkg-config"]).pkgconfig(&["pixman-1", "xkbcommon", "libdrm"]));
    r.push(recipe("hyprland-qt-support", &h("hyprland-qt-support"), cm).rev("v0.1.0").deps(&["hyprlang"])
        .note("Needs Qt 6 on the machine. Its QML module is installed inside the package (by default it asks\nfor Qt's own system directory).").tools(&["pkg-config"])
        .args(&["-DINSTALL_QML_PREFIX=lib/qt6/qml"]));
    r.push(recipe("hyprland-qtutils", &h("hyprland-qtutils"), cm).rev("v0.1.5").deps(&["hyprutils", "hyprland-qt-support"])
        .note("Needs Qt 6 on the machine. Finds hyprland-qt-support's QML module through QML_IMPORT_PATH.").tools(&["pkg-config"])
        .run(Run { isolate: Some("none".into()), env: vec![("QML_IMPORT_PATH".into(), "{dep:hyprland-qt-support}/lib/qt6/qml".into())], ..Default::default() }));
    r.push(recipe("hyprsysteminfo", &h("hyprsysteminfo"), cm).rev("v0.2.0").deps(&["hyprtoolkit", "hyprutils", "glaze", "pciutils"])
        .tools(&["pkg-config"]).pkgconfig(&["libdrm", "pixman-1"]));

    // ---------------------------------------------------------------- Wayland desktop tools (Meson, mostly)
    r.push(recipe("scdoc", "https://git.sr.ht/~sircmpwn/scdoc", "custom").rev("1.11.5")
        .note("Man-page compiler. A build-time tool for projects (fuzzel) whose build insists on their documentation.")
        .steps(&[r#"make -j"$FORGE_JOBS" CC=gcc PREFIX="$FORGE_PREFIX""#, r#"make install CC=gcc PREFIX="$FORGE_PREFIX" DESTDIR="$FORGE_STAGE""#]));
    r.push(recipe("tllist", &format!("{CB}dnkl/tllist"), "meson").rev("1.1.0").note("Header-only list library used by foot, fcft and fuzzel."));
    r.push(recipe("fcft", &format!("{CB}dnkl/fcft"), "meson").rev("3.3.3").deps(&["tllist", "utf8proc"]).note("Font loading and glyph rasterization library for foot and fuzzel.")
        .tools(&["pkg-config"]).pkgconfig(&["freetype2", "harfbuzz", "fontconfig", "pixman-1"]));
    r.push(recipe("foot", &format!("{CB}dnkl/foot"), "meson").rev("1.28.0").deps(&["fcft", "tllist", "utf8proc"])
        .note("Wayland terminal. It starts your shell, so only its own --config is pointed at the package\n(an XDG redirect would leak into every program you run in it).")
        .tools(&["pkg-config", "wayland-scanner"]).pkgconfig(&["wayland-client", "wayland-cursor", "wayland-protocols", "xkbcommon", "fontconfig", "pixman-1"])
        .args(&["-Ddocs=disabled", "-Dsystemd-units-dir=lib/systemd/user"]).run(spawner("foot", "config/foot/foot.ini", "--config")));
    r.push(recipe("fuzzel", &format!("{CB}dnkl/fuzzel"), "meson").rev("1.15.0").deps(&["fcft", "tllist"]).build_deps(&["scdoc"])
        .note("Application launcher. It starts programs, so only its own --config is pointed at the package.")
        .tools(&["pkg-config", "wayland-scanner"]).pkgconfig(&["wayland-client", "wayland-cursor", "wayland-protocols", "xkbcommon", "fontconfig", "pixman-1", "cairo"])
        .run(spawner("fuzzel", "config/fuzzel/fuzzel.ini", "--config")));
    r.push(recipe("wl-clipboard", &format!("{GH}bugaevc/wl-clipboard"), "meson").rev("v2.3.0").note("wl-copy and wl-paste: clipboard access from the command line (the tools gh looks for).")
        .tools(&["pkg-config", "wayland-scanner"]).pkgconfig(&["wayland-client", "wayland-protocols"])
        .args(&["-Dzshcompletiondir=share/zsh/site-functions", "-Dfishcompletiondir=share/fish/vendor_completions.d"]));
    r.push(recipe("cliphist", &format!("{GH}sentriz/cliphist"), "go").rev("v0.7.0").deps(&["wl-clipboard"]).note("Clipboard history. Uses wl-clipboard at runtime.").targets(&["."]));
    r.push(recipe("grim", &format!("{GH}emersion/grim"), "meson").rev("v1.4.0").note("Screenshots.").tools(&["pkg-config", "wayland-scanner"])
        .pkgconfig(&["libpng", "libjpeg", "pixman-1", "wayland-client", "wayland-protocols"]).args(&[&meson_man[..], &nofish[..]].concat()));
    r.push(recipe("slurp", &format!("{GH}emersion/slurp"), "meson").rev("v1.5.0").note("Select a region of the screen (pairs with grim).").tools(&["pkg-config", "wayland-scanner"])
        .pkgconfig(&["cairo", "wayland-client", "wayland-cursor", "wayland-protocols", "xkbcommon"]).args(&meson_man));
    r.push(recipe("mako", &format!("{GH}emersion/mako"), "meson").rev("v1.11.0").note(&format!("Notification daemon. Starts programs on actions, so only --config is pointed at the package.\n{sdbus_note}"))
        .tools(&["pkg-config", "wayland-scanner"]).pkgconfig(&["cairo", "pango", "pangocairo", "glib-2.0", "gobject-2.0", "wayland-client", "wayland-protocols", "wayland-cursor", "gdk-pixbuf-2.0"])
        .args(&meson_man).args_sh(vec![format!("-Dsd-bus-provider={}", sd("auto", "libelogind"))]).run(spawner("mako", "config/mako/config", "--config")));
    r.push(recipe("swaybg", &format!("{GH}swaywm/swaybg"), "meson").rev("v1.2.2").note("Wallpaper.").tools(&["pkg-config", "wayland-scanner"])
        .pkgconfig(&["wayland-client", "wayland-protocols", "cairo", "gdk-pixbuf-2.0"]).args(&meson_man));
    r.push(recipe("swaylock", &format!("{GH}swaywm/swaylock"), "meson").rev("v1.8.6").note("Screen locker. Uses the machine's PAM and needs a PAM service file for `swaylock`.")
        .tools(&["pkg-config", "wayland-scanner"]).pkgconfig(&["wayland-client", "wayland-protocols", "xkbcommon", "cairo", "gdk-pixbuf-2.0"]).args(&[&meson_man[..], &nocomp[..]].concat())
        .run(spawner("swaylock", "config/swaylock/config", "--config")));
    r.push(recipe("swayidle", &format!("{GH}swaywm/swayidle"), "meson").rev("v1.9.1").note(&format!("Idle manager. Runs commands, so only its own config is pointed at the package.\n{sdbus_note}"))
        .tools(&["pkg-config", "wayland-scanner"]).pkgconfig(&["wayland-client", "wayland-protocols", "wayland-server"]).args(&[&meson_man[..], &nocomp[..]].concat())
        .args_sh(vec![format!("-Dlogind-provider={}", sd("systemd", "elogind"))]).run(spawner("swayidle", "config/swayidle/config", "-C")));
    r.push(recipe("xwayland-satellite", &format!("{GH}Supreeeme/xwayland-satellite"), "cargo").rev("v0.8")
        .note("Runs X11 programs under compositors without built-in XWayland (such as niri).\nNeeds the `Xwayland` binary on the machine.")
        .tools(&["pkg-config"]).pkgconfig(&["xcb", "xcb-cursor"]));
    r
}

fn main() -> std::io::Result<()> {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../recipes");
    // Hand-written recipes that this program does not own.
    let keep = ["niri", "github-cli"];
    let recipes = all();
    for r in &recipes {
        assert!(!keep.contains(&r.name.as_str()), "{}", r.name);
        fs::write(out.join(format!("{}.toml", r.name)), r.render())?;
    }
    println!("wrote {} recipes; hand-written kept: {keep:?}", recipes.len());
    if std::env::args().any(|a| a == "--names") {
        println!("{}", recipes.iter().map(|r| r.name.as_str()).collect::<Vec<_>>().join(" "));
    }
    Ok(())
}
