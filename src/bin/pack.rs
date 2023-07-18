use std::fs;

use std::collections::BTreeMap;
use std::io::BufReader;
use std::io::Error;
use std::io::Read;
use std::path::PathBuf;
use std::str;
use std::str::FromStr;

use tracing::{error, info, warn};

use ffs::config::Config;
use ffs::config::{ERROR_STATUS_CLI, ERROR_STATUS_FUSE};
use ffs::format;
use ffs::time_ns;
use format::json::Value as JsonValue;
use format::toml::Value as TomlValue;
use format::yaml::Value as YamlValue;
use format::Format;
use format::Nodelike;
use format::Typ;

use ::xattr;

#[allow(dead_code)]
#[allow(unused_variables)]
fn pack<V>(path: PathBuf, config: &Config) -> std::io::Result<V>
where
    V: Nodelike + std::fmt::Display + Default,
{
    // get type of directory or file by xattr
    let path_type = match xattr::get(&path, "user.type") {
        Ok(optional_xattr) => {
            if config.allow_xattr {
                optional_xattr.unwrap_or(b"detect".to_vec())
            } else {
                b"detect".to_vec()
            }
        },
        Err(_) => {
            // Cannot call xattr::get on ._ file
            warn!("._ files, like {:?}, prevent xattr calls. It will be encoded in base64.", path);
            b"bytes".to_vec()
        }
    };
    info!("packing {:?} of path type {:?}", path, path_type);
    let mut path_type = str::from_utf8(&path_type).unwrap();

    // resolve type if it is 'detect'
    if path_type == "detect" {
        let path_file_type = path.metadata().unwrap().file_type();

        if path_file_type.is_file() {
            path_type = "auto";
        } else if path_file_type.is_dir() {
            let try_parsing_to_int = fs::read_dir(path.clone())?
                .map(|res| res.map(|e| e.path()))
                .map(|e| e.unwrap().file_name().unwrap().to_str().unwrap().parse::<u32>())
                .collect::<Result<Vec<_>,_>>();
            info!("parsed names or parse error: {:?}", try_parsing_to_int);
            match try_parsing_to_int {
                Ok(mut parsed_ints) => {
                    parsed_ints.sort_unstable();
                    let mut i = 0;
                    for parsed_int in parsed_ints.clone() {
                        if parsed_int != i {
                            info!("file {} is missing from the range of the number of files [0,{})", i, parsed_ints.len() as u32);
                            path_type = "map";
                            break;
                        }
                        i += 1;
                    }
                    if i == parsed_ints.len() as u32 {
                        path_type = "list";
                    }
                }
                Err(_) => {
                    path_type = "named";
                }
            }
        } else if path_file_type.is_symlink() {
            // TODO (nad) 2023-05-21 implement symlink support
            // is the exit status code name appropriate?
            error!("symlink file {:?} are not supported yet!", path.display());
            std::process::exit(ERROR_STATUS_FUSE);
        } else {
            error!("{:?} has unknown type and it is an unsupported file type (i.e. not file, directory, or symlink).", path.display());
            std::process::exit(ERROR_STATUS_FUSE);
        }
    }

    info!("type of {:?} as {}", path.display(), path_type);

    match path_type {
        "named" => {
            let mut children = fs::read_dir(path.clone())?
                .map(|res| res.map(|e| e.path()))
                .collect::<Result<Vec<_>, Error>>()?;
            children.sort_unstable_by(|a,b| a.file_name().cmp(&b.file_name()));

            // println!("{:?}", children);
            let mut entries = BTreeMap::new();

            for child in &children {
                let child_name = child.file_name().unwrap().to_str().unwrap();
                // println!("looking at: {}", child_name);
                if config.ignored_file(child_name) {
                    warn!("skipping ignored file '{:?}'", child_name);
                    continue;
                }
                let name: String;
                match xattr::get(&child, "user.original_name") {
                    Ok(Some(original_name)) => {
                        let old_name = str::from_utf8(&original_name).unwrap();
                        if !config.valid_name(old_name) {
                            // original name must have been munged, so restore original
                            name = old_name.to_string();
                        } else {
                            // original name wasn't munged, keep the current name
                            // in case it was renamed
                            name = child_name.to_string();
                        }
                    },
                    Ok(_) | Err(_) => {
                        // use current name because xattr is None or getting xattr on files (like ._ files) errors
                        name = child_name.to_string();
                    }
                }
                let value = pack(child.clone(), &config)?;
                entries.insert(name, value);
            }

            Ok(V::from_named_dir(entries, &config))
        }
        "list" => {
            let mut children = fs::read_dir(path.clone())?
                .map(|res| res.map(|e| e.path()))
                .collect::<Result<Vec<_>, Error>>()?;
            children.sort_unstable_by(|a,b| a.file_name().unwrap().to_str().unwrap()
                                      .cmp(&b.file_name().unwrap().to_str().unwrap()));

            let mut entries = Vec::with_capacity(children.len());

            for child in children {
                let name = child.file_name().unwrap().to_str().unwrap();
                if config.ignored_file(name) {
                    warn!("skipping ignored file '{:?}'", name);
                    continue;
                }
                let value = pack(child, &config)?;
                entries.push(value);
            }

            Ok(V::from_list_dir(entries, &config))
        }
        typ => {
            if let Ok(t) = Typ::from_str(typ) {
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
                error!("Very bad error. Received undetected and unknown type '{}' for file '{:?}'", typ, path.display());
                std::process::exit(ERROR_STATUS_FUSE);
            }
        }
    }
}


fn main() -> std::io::Result<()> {
    let config = Config::from_pack_args();
    info!("received config: {:?}", config);

    let mount = match &config.mount {
        Some(mount) => mount,
        None => {
            error!("Cannot pack unspecified directory.");
            std::process::exit(ERROR_STATUS_CLI);
        }
    };

    let folder = PathBuf::from(mount);

    let writer = match config.output_writer() {
        Some(writer) => writer,
        None => return Ok(()),
    };

    match &config.output_format {
        Format::Json => {
            let v: JsonValue = time_ns!("saving", pack(folder, &config)?, config.timing);

            time_ns!("writing", v.to_writer(writer, config.pretty), config.timing);
        }
        Format::Toml => {
            let v: TomlValue = time_ns!("saving", pack(folder, &config)?, config.timing);

            time_ns!("writing", v.to_writer(writer, config.pretty), config.timing);
        }
        Format::Yaml => {
            let v: YamlValue = time_ns!("saving", pack(folder, &config)?, config.timing);

            time_ns!("writing", v.to_writer(writer, config.pretty), config.timing);
        }
    }

    Ok(())
}
