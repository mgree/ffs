use std::fs;

use std::ffi::OsString;
use std::collections::VecDeque;
use std::collections::HashMap;
use std::io::Read;
use std::io::BufReader;
use std::io::Write;
use std::io::Error;
use std::path::PathBuf;
use std::str;
use std::str::FromStr;

use tracing::warn;

use ffs::config::Config;
// use ffs::config::Output;
use ffs::format;
use ffs::time_ns;
use format::Format;
use format::Nodelike;
use format::Typ;
use format::json::Value as JsonValue;
use format::toml::Value as TomlValue;
use format::yaml::Value as YamlValue;

use ::xattr;

#[allow(dead_code)]
#[allow(unused_variables)]
fn pack<V>(path: PathBuf, config: &Config) -> std::option::Option<(OsString,V)>
where
    V: Nodelike + std::fmt::Display + Default,
{
    let path_type = xattr::get(&path, "user.type").unwrap().unwrap();
    let path_type = str::from_utf8(&path_type).unwrap();
    println!("{:?} is a {}", path, path_type);

    // TODO (nad) 2023-03-22 get the original name from the xattr
    match path_type {
        "map" => {
            let mut children = fs::read_dir(path.clone()).unwrap()
                .map(|res| res.map(|e| e.path()))
                .collect::<Result<Vec<_>, Error>>().unwrap();


            // return something for now
            // TODO: fix this
            Some((path.as_os_str().to_os_string(), V::default()))
            // let v = pack(path.clone(), config)?;
            // (path.file_name().unwrap().to_os_string(), v)
        }
        "list" => {
            let mut children = fs::read_dir(path.clone()).unwrap()
                .map(|res| res.map(|e| e.path()))
                .collect::<Result<Vec<_>, Error>>().unwrap();


            // return something for now
            // TODO: fix this
            Some((path.as_os_str().to_os_string(), V::default()))
            // let v = pack(child.clone(), config)?;
            // (child.file_name().unwrap().to_os_string(), v)
        }
        typ => {
            if let Ok(t) = Typ::from_str(typ) {
                let mut name = path.file_name().unwrap().to_os_string();
                if config.ignored_file(name.to_str().unwrap()) {
                    warn!("skipping ignored file '{:?}'", name);
                    return None
                };
                println!("parsed type {}", t);
                let file = fs::File::open(&path).unwrap();
                let mut reader = BufReader::new(&file);
                let mut contents: Vec<u8> = Vec::new();
                reader.read_to_end(&mut contents).unwrap();
                match String::from_utf8(contents.clone()) {
                    Ok(mut contents) if t != Typ::Bytes => {
                        if config.add_newlines && contents.ends_with('\n') {
                            contents.truncate(contents.len() - 1);
                        }
                        match config.munge {
                            ffs::config::Munge::Rename => {
                                let original = xattr::get(&name, "user.original_name");
                                if let Ok(Some(original)) = original {
                                    let original = str::from_utf8(&original).unwrap();
                                    name = OsString::from(original);
                                }
                            }
                            ffs::config::Munge::Filter => {

                            }
                        }
                        Some((name, V::from_string(t, contents, &config)))
                    }
                    Ok(_) | Err(_) => {
                        Some((name, V::from_bytes(contents, &config)))
                    }
                }
            } else {
                // we were already supposed to detect dirs with map and list
                // so this is a bad error.
                panic!("Very bad error. Unknown type {}", typ);
            }
        }
    }

    // let parsed_value = match dir_type {
    //     "map" => {
    //         let mut entries = HashMap::with_capacity(queue.len());
    //         let files = queue
    //             .into_iter()
    //             .map(|(name, entry)| (name.clone().into_string().unwrap(), entry/*, insert original_name */))
    //             .collect::<Vec<_>>();
    //         for (name, value/*, original_name */) in files {
    //             if config.ignored_file(name.as_str()) {
    //                 warn!("skipping ignored file '{}'", name);
    //                 continue;
    //             }
    //             // let name = original_name.as_ref().unwrap_or(name).into();
    //             entries.insert(name, value);
    //         }
    //         V::from_named_dir(entries, &config)
    //     }
    //     "list" => {
    //         let mut entries = Vec::with_capacity(queue.len());
    //         let mut files = queue
    //             .into_iter()
    //             .map(|(name, entry)| (name.clone().into_string().unwrap(), entry))
    //             .collect::<Vec<_>>();
    //         files.sort_unstable_by(|(name1, _), (name2, _)| name1.cmp(name2));
    //         for (name, value) in files {
    //             if config.ignored_file(&name) {
    //                 warn!("skipping ignored file '{}'", name);
    //                 continue;
    //             }
    //             entries.push(value);
    //         }
    //         V::from_list_dir(entries, &config)
    //     }
    //     _ => {
    //         panic!("Unknown directory type: {}", dir_type);
    //     }
    // };
}


fn main() -> std::io::Result<()> {
    let config = Config::from_pack_args();
    /* To pack
     * 1. get the path of the unpacked folder from config
     * 2. get the format from config
     * 3. call pack on path of root
     * 4. use to_writer to write to the output file given by config
     */

    // test code that should be run after unpacking invoice.yaml in the cwd.
    let mount = match &config.mount {
        Some(m) => m,
        None => {
            println!("No mount point given");
            return Ok(());
        }
    };


    let folder = PathBuf::from(mount);

    let writer = match config.output_writer() {
        Some(writer) => writer,
        None => return Ok(()),
    };

    println!("output format: {:?}", &config.output_format);

    match &config.output_format {
        Format::Json => {
            let v: JsonValue = pack(folder, &config).unwrap().1;
            v.to_writer(writer, config.pretty);
        }
        Format::Toml => {
            let v: TomlValue = pack(folder, &config).unwrap().1;
            v.to_writer(writer, config.pretty);
        }
        Format::Yaml => {
            let v: YamlValue = pack(folder, &config).unwrap().1;
            v.to_writer(writer, config.pretty);
        }
    }

    Ok(())
}
