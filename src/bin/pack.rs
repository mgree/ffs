use std::fs;

use std::collections::BTreeMap;
use std::io::Read;
use std::io::BufReader;
use std::io::Error;
use std::path::PathBuf;
use std::str;
use std::str::FromStr;

use tracing::warn;
use tracing::info;

use ffs::format;
use ffs::time_ns;
use ffs::config::Config;
use ffs::config::Input;
use format::Format;
use format::Nodelike;
use format::Typ;
use format::json::Value as JsonValue;
use format::toml::Value as TomlValue;
use format::yaml::Value as YamlValue;

use ::xattr;

#[allow(dead_code)]
#[allow(unused_variables)]
fn pack<V>(path: PathBuf, config: &Config) -> std::io::Result<V>
where
    V: Nodelike + std::fmt::Display + Default,
{
    let path_type = xattr::get(&path, "user.type").unwrap().unwrap();
    let path_type = str::from_utf8(&path_type).unwrap();
    // println!("{:?} is a {}", path, path_type);

    match path_type {
        "map" => {
            let mut children = fs::read_dir(path.clone()).unwrap()
                .map(|res| res.map(|e| e.path()))
                .collect::<Result<Vec<_>, Error>>().unwrap();
            children.sort_unstable_by(|a,b| a.file_name().cmp(&b.file_name()));

            let mut entries = BTreeMap::new();

            for child in &children {
                let child_name = child.file_name().unwrap();
                let name = child_name.to_str().unwrap();
                if config.ignored_file(name) {
                    warn!("skipping ignored file '{:?}'", name);
                    continue
                };
                let original_name = xattr::get(&child, "user.original_name")?.unwrap();
                let v = pack(child.clone(), &config)?;
                let name = str::from_utf8(&original_name).unwrap_or(&name);
                entries.insert(name.to_string(), v);
            }

            Ok(V::from_named_dir(entries, &config))
        }
        "list" => {
            let mut children = fs::read_dir(path.clone()).unwrap()
                .map(|res| res.map(|e| e.path()))
                .collect::<Result<Vec<_>, Error>>().unwrap();
            children.sort_unstable_by(|a,b| a.file_name().cmp(&b.file_name()));

            let mut entries = Vec::with_capacity(children.len());

            for child in children {
                let name = child.file_name().unwrap().to_str().unwrap();
                if config.ignored_file(&name) {
                    warn!("skipping ignored file '{}'", name);
                    continue
                };
                let v = pack(child, &config)?;
                entries.push(v);
            }

            Ok(V::from_list_dir(entries, &config))
        }
        typ => {
            if let Ok(t) = Typ::from_str(typ) {
                // println!("parsed type {}", t);
                let file = fs::File::open(&path).unwrap();
                let mut reader = BufReader::new(&file);
                let mut contents: Vec<u8> = Vec::new();
                reader.read_to_end(&mut contents).unwrap();
                match String::from_utf8(contents.clone()) {
                    Ok(mut contents) if t != Typ::Bytes => {
                        if config.add_newlines && contents.ends_with('\n') {
                            contents.truncate(contents.len() - 1);
                        }
                        Ok(V::from_string(t, contents, &config))
                    }
                    Ok(_) | Err(_) => {
                        Ok(V::from_bytes(contents, &config))
                    }
                }
            } else {
                // we were already supposed to detect dirs with map and list
                // so this is a bad error.
                panic!("Very bad error. Unknown type {}", typ);
            }
        }
    }
}


fn main() -> std::io::Result<()> {
    let config = Config::from_pack_args();

    // println!("{:?}", &config);

    let mount = match &config.input {
        Input::File(mount) => mount,
        _ => {
            panic!("input must be a file path");
        }
    };

    let folder = PathBuf::from(mount);

    let writer = match config.output_writer() {
        Some(writer) => writer,
        None => return Ok(()),
    };

    // println!("output format: {:?}", &config.output_format);

    match &config.output_format {
        Format::Json => {
            let v: JsonValue = time_ns!(
                "saving",
                pack(folder, &config).unwrap(),
                config.timing
            );

            time_ns!(
                "writing",
                v.to_writer(writer, config.pretty),
                config.timing
            );
        }
        Format::Toml => {
            let v: TomlValue = time_ns!(
                "saving",
                pack(folder, &config).unwrap(),
                config.timing
            );

            time_ns!(
                "writing",
                v.to_writer(writer, config.pretty),
                config.timing
            );
        }
        Format::Yaml => {
            let v: YamlValue = time_ns!(
                "saving",
                pack(folder, &config).unwrap(),
                config.timing
            );

            time_ns!(
                "writing",
                v.to_writer(writer, config.pretty),
                config.timing
            );
        }
    }

    Ok(())
}
