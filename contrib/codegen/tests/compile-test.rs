extern crate compiletest_rs as compiletest;

use std::path::{Path, PathBuf};
use std::{io, fs::Metadata, time::SystemTime};

#[derive(Copy, Clone)]
enum Kind {
    #[allow(dead_code)]
    Dynamic,
    Static
}

impl Kind {
    fn extension(self) -> &'static str {
        match self {
            #[cfg(windows)] Kind::Dynamic => ".dll",
            #[cfg(all(unix, target_os = "macos"))] Kind::Dynamic => ".dylib",
            #[cfg(all(unix, not(target_os = "macos")))] Kind::Dynamic => ".so",
            Kind::Static => ".rlib"
        }
    }
}

fn target_path() -> PathBuf {
    #[cfg(debug_assertions)] const ENVIRONMENT: &str = "debug";
    #[cfg(not(debug_assertions))] const ENVIRONMENT: &str = "release";

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().parent().unwrap()
        .join("target")
        .join(ENVIRONMENT)
}

fn link_flag(flag: &str, lib: &str, rel_path: &[&str]) -> String {
    let mut path = target_path();
    for component in rel_path {
        path = path.join(component);
    }

    format!("{} {}={}", flag, lib, path.display())
}

fn best_time_for(metadata: &Metadata) -> SystemTime {
    metadata.created()
        .or_else(|_| metadata.modified())
        .or_else(|_| metadata.accessed())
        .unwrap_or_else(|_| SystemTime::now())
}

fn extern_dep(name: &str, kind: Kind) -> io::Result<String> {
    let deps_root = target_path().join("deps");
    let dep_name = format!("lib{}", name);

    let mut dep_path: Option<PathBuf> = None;
    for entry in deps_root.read_dir().expect("read_dir call failed") {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue
        };

        let filename = entry.file_name();
        let filename = filename.to_string_lossy();
        let lib_name = filename.split('.').next().unwrap().split('-').next().unwrap();

        if lib_name == dep_name && filename.ends_with(kind.extension()) {
            if let Some(ref mut existing) = dep_path {
                if best_time_for(&entry.metadata()?) > best_time_for(&existing.metadata()?) {
                    *existing = entry.path().into();
                }
            } else {
                dep_path = Some(entry.path().into());
            }
        }
    }

    let dep = dep_path.ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    let filename = dep.file_name().ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))?;
    Ok(link_flag("--extern", name, &["deps", &filename.to_string_lossy()]))
}

fn run_mode(mode: &'static str, path: &'static str) {
    let mut config = compiletest::Config::default();
    config.mode = mode.parse().expect("invalid mode");
    config.src_base = format!("tests/{}", path).into();
    config.clean_rmeta();

    config.target_rustcflags = Some([
        link_flag("-L", "crate", &[]),
        link_flag("-L", "dependency", &["deps"]),
        extern_dep("rocket_http", Kind::Static).expect("find http dep"),
        extern_dep("rocket", Kind::Static).expect("find core dep"),
        extern_dep("rocket_contrib", Kind::Static).expect("find contrib dep"),
    ].join(" "));

    compiletest::run_tests(&config);
}

#[test]
fn compile_test() {
    run_mode("ui", "ui-fail");
    run_mode("compile-fail", "ui-fail");
}
