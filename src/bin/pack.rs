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
fn pack<V>(dir: PathBuf, config: &Config) -> std::io::Result<V>
where
    V: Nodelike + std::fmt::Display + Default,
{
    /*
     * get xattr of dir (this function only gets recursively called on directories)
     *
     * loop through the entries in this directory (sorted?)
     * - if the entry is a list or map, recursively call this function first. "depth first"
     *   append the returned Value to the vec along with the path
     * - else append it to the vec with the path and value parsed by xattr and V type.
     *
     * with the list of (path, value) pairs, build the Value of this list or map from the xattr
     * determined in the beginning
     */

    let dir_type = xattr::get(&dir, "user.type")?.unwrap();
    let dir_type = str::from_utf8(&dir_type).unwrap();
    println!("parsing dir {:?} of type {}", dir, dir_type);

    let mut children = fs::read_dir(dir)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, Error>>()?;
    children.sort();

    let mut queue: Vec<(OsString, V)> = Vec::new();

    for child in children {
        println!("child path: {:?}", &child.display());
        let child_type = xattr::get(&child, "user.type")?.unwrap();
        let child_type = str::from_utf8(&child_type).unwrap();
        println!("child_type: {}", child_type);
        // TODO (nad) 2023-03-22 get the original name from the xattr
        match child_type {
            "map" => {
                // println!("map");
                let v = pack(child.clone(), config)?;
                queue.push((child.file_name().unwrap().to_os_string(), v));
            }
            "list" => {
                // println!("list");
                let v = pack(child.clone(), config)?;
                queue.push((child.file_name().unwrap().to_os_string(), v));
            }
            typ => {
                if let Ok(t) = Typ::from_str(typ) {
                    println!("parsed type {}", t);
                    let file = fs::File::open(&child).unwrap();
                    let mut reader = BufReader::new(&file);
                    let mut contents: Vec<u8> = Vec::new();
                    reader.read_to_end(&mut contents)?;
                    match String::from_utf8(contents.clone()) {
                        Ok(mut contents) if t != Typ::Bytes => {
                            if config.add_newlines && contents.ends_with('\n') {
                                contents.truncate(contents.len() - 1);
                            }
                            // TODO 2021-06-24 trim?
                            queue.push((child.file_name().unwrap().to_os_string(), V::from_string(t, contents, &config)));
                        }
                        Ok(_) | Err(_) => {
                            queue.push((child.file_name().unwrap().to_os_string(), V::from_bytes(contents, &config)));
                        }
                    };
                } else {
                    // we were already supposed to detect dirs with map and list
                    // so this is a bad error.
                    panic!("Very bad error. Unknown type {}", typ);
                }
            }
        };
    }

    // we now have the complete list of file names and child values in the queue.
    let parsed_value = match dir_type {
        "map" => {
            let mut entries = HashMap::with_capacity(queue.len());
            let files = queue
                .iter()
                .map(|(name, entry)| (name.clone().into_string().unwrap(), entry/*, insert original_name */))
                .collect::<Vec<_>>();
            for (name, value/*, original_name */) in files.iter() {
                if config.ignored_file(name) {
                    warn!("skipping ignored file '{}'", name);
                    continue;
                }
                // let name = original_name.as_ref().unwrap_or(name).into();
                entries.insert(name, value);
            }
            V::from_named_dir(entries, &config)
        }
        "list" => {
            let mut entries = Vec::with_capacity(queue.len());
            let mut files = queue
                .iter()
                .map(|(name, entry)| (name.clone().into_string().unwrap(), entry))
                .collect::<Vec<_>>();
            files.sort_unstable_by(|(name1, _), (name2, _)| name1.cmp(name2));
            for (name, value) in files {
                if config.ignored_file(&name) {
                    warn!("skipping ignored file '{}'", name);
                    continue;
                }
                entries.push(value);
            }
            V::from_list_dir(entries, &config)

        }
        _ => {
            panic!("Unknown directory type: {}", dir_type);
        }
    };

    Ok(parsed_value)
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

    match &config.output_format {
        Format::Json => {
            let v: JsonValue = pack(folder, &config).unwrap();
            v.to_writer(writer, config.pretty);
        }
        Format::Toml => {
            let v: TomlValue = pack(folder, &config).unwrap();
            v.to_writer(writer, config.pretty);
        }
        Format::Yaml => {
            let v: YamlValue = pack(folder, &config).unwrap();
            v.to_writer(writer, config.pretty);
        }
    }

    Ok(())
}
