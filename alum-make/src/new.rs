use crate::config::{Build, Config, Package};
use std::{
    error::Error,
    fs::{self, create_dir},
};

pub fn new(
    name: String,
    language: String,
    cc: Option<String>,
    alc: Option<String>,
    linker: String,
) -> Result<(), Box<dyn Error>> {
    let config = Config {
        package: Package {
            name: name.clone(),
            version: "0.1.0".to_string(),
            author: "".to_string(),
            license: "No License".to_string(),
            language: language.clone(),
        },
        build: Build {
            linker,
            cc,
            cflags: None,
            alc,
            alflags: None,
            lnflags: None,
            includes: None,
            nostdlib: None,
            library_type: None,
        },
        native: None,
        dependencies: None,
    };
    let toml_string = toml::to_string_pretty(&config)?;
    create_dir(name.clone())?;
    create_dir(format!("{}/src", name.clone()))?;
    match language.as_str() {
        "C" => {
            fs::write(
                format!("{}/src/main.c", name),
                "#include <stdio.h>\n\nint main() {\n    printf(\"Hello, World!\\n\");\n    return 0;\n}\n",
            )?;
        }
        "alum" => {
            fs::write(
                format!("{}/src/main.al", name),
                "import io\nusing io::{write, read, print, println, input, fopen, fclose, fread, fwrite, lseek, pipe, pipe2, dup, dup2, dup3}\n\nfun main(): int {\n    println(\"Hello, World!\");\n    return 0;\n}\n",
            )?;
        }
        _ => {
            return Err(format!("Unsupported language: {:?}", language).into());
        }
    }
    fs::write(format!("{}/Alumake.toml", name), toml_string)?;
    Ok(())
}
