use std::os::unix::fs::symlink;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{language_query, read_markers, LANGUAGES};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap()
}

fn sandbox(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = env::temp_dir().join(format!(
        "hx-highlight-{name}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn skip_or_require(programs: &[&str], test: &str) -> bool {
    let missing = programs
        .iter()
        .filter(|program| Command::new(program).arg("--version").output().is_err())
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return true;
    }
    let reason = format!("{} not installed", missing.join(", "));
    if env::var_os("LAND_GATE").is_some() || env::var_os("CI").is_some() {
        panic!("{test}: {reason}");
    }
    if let Some(path) = env::var_os("TEST_SKIP_LOG") {
        use std::io::Write;
        writeln!(
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .unwrap(),
            "{test}: {reason}"
        )
        .unwrap();
    }
    false
}

fn helix_runtime() -> PathBuf {
    if let Some(runtime) = env::var_os("HELIX_RUNTIME") {
        let path = PathBuf::from(runtime);
        if path.join("queries/rust/highlights.scm").is_file() {
            return path;
        }
    }

    let hx = Command::new("which")
        .arg("hx")
        .output()
        .expect("which hx")
        .stdout;
    let executable = PathBuf::from(String::from_utf8(hx).unwrap().trim())
        .canonicalize()
        .expect("resolve hx executable");
    let prefix = executable.parent().unwrap().parent().unwrap();
    [
        executable.parent().unwrap().join("runtime"),
        prefix.join("lib/runtime"),
        prefix.join("lib/helix/runtime"),
        prefix.join("share/helix/runtime"),
        PathBuf::from("/usr/share/helix/runtime"),
        PathBuf::from("/usr/local/share/helix/runtime"),
    ]
    .into_iter()
    .find(|path| path.join("queries/rust/highlights.scm").is_file())
    .or_else(|| {
        fs::read_dir("/nix/store")
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with("-helix-runtime"))
                    && path.join("queries/rust/highlights.scm").is_file()
            })
    })
    .expect("could not locate bundled Helix runtime queries")
}

pub(super) fn runtime_with_overrides(base: &Path, overrides: &Path, destination: &Path) {
    fs::create_dir_all(destination.join("queries")).unwrap();
    for entry in fs::read_dir(base).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() != "queries" {
            symlink(entry.path(), destination.join(entry.file_name())).unwrap();
        }
    }
    for entry in fs::read_dir(base.join("queries")).unwrap() {
        let entry = entry.unwrap();
        let lang = entry.file_name();
        if lang == "comment" || lang == "org" {
            let lang_dir = destination.join("queries").join(&lang);
            fs::create_dir_all(&lang_dir).unwrap();
            fs::copy(
                overrides.join(&lang).join("highlights.scm"),
                lang_dir.join("highlights.scm"),
            )
            .unwrap();
        } else {
            symlink(entry.path(), destination.join("queries").join(lang)).unwrap();
        }
    }
}

/// The shipped queries embed the bundled ones of the Helix release in
/// runtime/HELIX_VERSION. Only that release can regenerate them exactly; the
/// gate (LAND_GATE or CI) must run it.
fn installed_helix_is_pinned(test: &str) -> bool {
    if !skip_or_require(&["hx"], test) {
        return false;
    }
    let pinned = fs::read_to_string(root().join("runtime/HELIX_VERSION")).unwrap();
    let out = Command::new("hx").arg("--version").output().unwrap();
    let installed = String::from_utf8_lossy(&out.stdout).into_owned();
    if installed.split_whitespace().nth(1) == Some(pinned.trim()) {
        return true;
    }
    let reason = format!("hx is {}, queries are pinned to {}", installed.trim(), pinned.trim());
    if env::var_os("LAND_GATE").is_some() || env::var_os("CI").is_some() {
        panic!("{test}: {reason}");
    }
    eprintln!("{test}: {reason}");
    false
}

#[test]
fn generated_language_queries_preserve_bundled_queries_and_do_not_shadow_rust() {
    const TEST: &str = "generated_language_queries_preserve_bundled_queries_and_do_not_shadow_rust";
    if !installed_helix_is_pinned(TEST) {
        return;
    }
    let repo = root();
    let query_root = repo.join("runtime/queries");
    let runtime = helix_runtime();
    let actual = fs::read_dir(&query_root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    let mut expected = LANGUAGES
        .iter()
        .map(|(lang, _)| (*lang).to_owned())
        .collect::<Vec<_>>();
    expected.sort();
    let mut actual_sorted = actual;
    actual_sorted.sort();
    assert_eq!(
        actual_sorted, expected,
        "generated query directories must only exist for languages with intentional additions"
    );

    let markers = read_markers(&repo.join("markers/markers.json")).unwrap();
    for (lang, nodes) in LANGUAGES {
        let bundled = fs::read_to_string(runtime.join(format!("queries/{lang}/highlights.scm")))
            .unwrap_or_else(|error| panic!("bundled {lang} query is unavailable: {error}"));
        let checked_in = fs::read_to_string(query_root.join(lang).join("highlights.scm")).unwrap();
        assert_eq!(
            checked_in,
            language_query(&bundled, lang, &markers, nodes),
            "{lang} query must be the verbatim bundled query followed only by generated marker additions"
        );
    }
}

#[test]
fn real_hx_rust_buffer_keeps_keyword_highlighting_and_red_markers() {
    const TEST: &str = "real_hx_rust_buffer_keeps_keyword_highlighting_and_red_markers";
    if !skip_or_require(&["hx", "tmux"], TEST) {
        return;
    }
    let repo = root();
    let runtime = helix_runtime();
    let dir = sandbox("rust-runtime");
    let home = dir.join("home");
    let config = dir.join("config/helix");
    let data = dir.join("data");
    fs::create_dir_all(&config).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&data).unwrap();

    fs::write(config.join("config.toml"), "theme = \"hx-highlight\"\n").unwrap();
    fs::create_dir_all(config.join("themes")).unwrap();
    fs::copy(
        repo.join("runtime/themes/hx-highlight.toml"),
        config.join("themes/hx-highlight.toml"),
    )
    .unwrap();
    let overrides = repo.join("runtime/queries");
    let isolated_runtime = dir.join("helix-runtime");
    runtime_with_overrides(&runtime, &overrides, &isolated_runtime);
    symlink(&isolated_runtime, config.join("runtime")).unwrap();

    let source = dir.join("sample.rs");
    fs::write(
        &source,
        "// TODO\n// BEAD\n// NOTE\nfn main() { let answer = true; }\n",
    )
    .unwrap();
    let socket = dir.join("tmux.sock");
    let tmux = |args: &[&str]| -> Output {
        Command::new("tmux")
            .args(["-S", socket.to_str().unwrap(), "-f", "/dev/null"])
            .args(args)
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", dir.join("config"))
            .env("XDG_DATA_HOME", &data)
            .output()
            .unwrap()
    };
    let launch = format!(
        "env HOME={} XDG_CONFIG_HOME={} XDG_DATA_HOME={} TERM=xterm-256color COLORTERM=truecolor hx {}",
        home.display(),
        dir.join("config").display(),
        data.display(),
        source.display()
    );
    let started = tmux(&["new-session", "-d", "-x", "100", "-y", "10", &launch]);
    assert!(
        started.status.success(),
        "tmux failed to start hx: {}",
        String::from_utf8_lossy(&started.stderr)
    );
    thread::sleep(Duration::from_secs(2));
    let captured = tmux(&["capture-pane", "-e", "-p"]);
    let _ = tmux(&["kill-server"]);
    assert!(
        captured.status.success(),
        "tmux could not capture hx: {}",
        String::from_utf8_lossy(&captured.stderr)
    );
    let screen = String::from_utf8_lossy(&captured.stdout);

    for marker in ["TODO", "BEAD", "NOTE"] {
        assert!(
            screen.contains(&format!("\x1b[38;5;15m\x1b[48;5;1m{marker}")),
            "{marker} should be white on red in a real Rust buffer:\n{screen}"
        );
    }
    let keyword = screen
        .find("fn")
        .expect("Rust keyword `fn` missing from rendered buffer");
    let before_keyword = &screen[..keyword];
    let last_line = before_keyword.rsplit('\n').next().unwrap_or(before_keyword);
    assert!(
        last_line.contains("\x1b[38;5;") || last_line.contains("\x1b[38;2;"),
        "Rust keyword should have a syntax color before it:\n{screen}"
    );
}
