extern crate bindgen;
extern crate cc;

use bindgen::callbacks::{MacroParsingBehavior, ParseCallbacks};
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
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

    // paho suggest to simply use `make` in the correct folder, this is what we are doing very in a
    // very simple and easy way. We log any error that may happen.
    // Another option is to use CMake, to generate the Makefile and then, use Make
    let compile_result = Command::new("make").current_dir("paho.mqtt.c").output();

    // The compilation didn't work out, maybe "make" is missing in the system?
    // We simply print the error returned by command.
    if compile_result.is_err() {
        println!("Error in using `make` to compile `paho.mqtt.c`");
        println!("{}", compile_result.unwrap_err());

        std::process::exit(101);
    }

    // We know that compile_result is an `Ok`, we simply unwrap.
    let compile_result = compile_result.unwrap();

    // the compilation may fail, if it fail we simply print both STDERR and STDOUT
    if !compile_result.status.success() {
        println!("Error during the compilation");
        println!(
            "STDERR: {}",
            String::from_utf8_lossy(&compile_result.stderr)
        );
        println!(
            "STDOUT: {}",
            String::from_utf8_lossy(&compile_result.stdout)
        );

        std::process::exit(102);
    }

    // the compilation was successful, we can move on

    // We check if the files where really compiled.
    let output_dir = Path::new("paho.mqtt.c/build/output/");
    let artifacts = vec![
        "libpaho-mqtt3a.so",
        "libpaho-mqtt3as.so",
        "libpaho-mqtt3c.so",
        "libpaho-mqtt3cs.so",
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
    // we simply canonicalize and unwrap, we are (reasonably) sure that the unwrap will
    // successed since we have actually build the path, check that exists, and any part of
    // what we build is a directory we are (reasonably) sure exists.
    println!(
        "cargo:rustc-link-search={}",
        output_dir.canonicalize().unwrap().to_string_lossy()
    );

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
