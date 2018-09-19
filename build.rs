extern crate bindgen;
extern crate cmake;

use bindgen::callbacks::{MacroParsingBehavior, ParseCallbacks};
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

// bindgen generate twice the IPPORT_RESERVED const, it is not so clear why.
// However this can be fixed using a custo `will_parse_macro` that need to be implemented agains a
// struct.
// The following struct and impl block do just this.
// The macro callback is used in bindgen at the end of the file.
#[derive(Debug)]
struct MacroCallback {
    macros: Arc<RwLock<HashSet<String>>>,
}

impl ParseCallbacks for MacroCallback {
    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        self.macros.write().unwrap().insert(name.into());

        if name == "IPPORT_RESERVED" {
            return MacroParsingBehavior::Ignore;
        }

        MacroParsingBehavior::Default
    }
}

// In the main function we will refer to the whole library `paho.mqtt.c` simply as `paho`
fn main() {
    // we rerun the build if the `build.rs` file is changed.
    println!("cargo:rerun-if-changed=build.rs");

    // We will use the directory `paho.mqtt.c`, if something change there we should rerun
    // the compile step.
    println!("cargo:rerun-if-changed=paho.mqtt.c/");

    // We use cmake that allow to create the static builds along with ssl
    let mut build_dst = cmake::Config::new("paho.mqtt.c/")
        .define("PAHO_BUILD_STATIC", "TRUE")
        .define("PAHO_WITH_SSL", "TRUE")
        .build();

    // We check if the files where really compiled.
    let output_dir = build_dst.clone();
    let output_dir = Path::new(&output_dir).join("lib");
    let artifacts = vec![
        "libpaho-mqtt3a.so",
        "libpaho-mqtt3as.so",
        "libpaho-mqtt3c.so",
        "libpaho-mqtt3cs.so",
        "libpaho-mqtt3a-static.a",
        "libpaho-mqtt3as-static.a",
        "libpaho-mqtt3c-static.a",
        "libpaho-mqtt3cs-static.a",
    ];
    for artifact in artifacts {
        let artifact = output_dir.join(Path::new(artifact));
        println!("Checking {}", artifact.to_string_lossy());
        if !artifact.exists() {
            println!(
                "Error, we should found `{}`, but it seems to don't be there!",
                artifact.to_string_lossy()
            );
            std::process::exit(103);
        }
    }

    // we add the folder where all the libraries are built to the path search
    build_dst.push("lib");
    println!("cargo:rustc-link-search=native={}", build_dst.display());

    // here we add the two libraries we just build, do not add the prefix `lib` and the postfix
    // `.so`
    // I believe is also possible to add the static ones.
    println!("cargo:rustc-link-lib=paho-mqtt3cs");
    println!("cargo:rustc-link-lib=paho-mqtt3as");
    let macros = Arc::new(RwLock::new(HashSet::new()));

    // The next step is to generate the bindings using bindgen
    let bindings = bindgen::Builder::default()
        .blacklist_type("IPPORT_.*")
        .header("wrapper.h")
        .parse_callbacks(Box::new(MacroCallback {
            macros: macros.clone(),
        })).generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
