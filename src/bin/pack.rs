// use std::collections::VecDeque;
use std::fs;
use std::io::Read;
use std::io::BufReader;
// use std::io::Write;
use std::path::PathBuf;
use std::str;
use std::str::FromStr;

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
use ::walkdir;
use walkdir::WalkDir;

#[allow(dead_code)]
#[allow(unused_variables)]
fn pack<V>(folder: PathBuf, config: &Config) -> std::io::Result<V>
where
    V: Nodelike + std::fmt::Display + Default,
{
    // TODO (nad) fix this
    // 1. change folder to path
    // 2. get the xattr of path.
    // 3. if it is of map or list, use walker of depth 1
    // 4. if it is a file, use the xattr to determine the type and read the file.
    // 5. after this function returns the highest level Value, use to_writer

    // let walker = WalkDir::new(folder).max_depth(1).into_iter();
    // for entry in walker {
    //     let e = entry?;
    //     let path = e.path();
    //     println!("{:?}", &path.display());
    //     let typ = xattr::get(path, "user.type")?.unwrap();
    //     let typ = str::from_utf8(&typ).unwrap();
    //     match Typ::from_str(typ) {
    //         Ok(t) => {
    //             let file = fs::File::open(&path).unwrap();
    //             let mut reader = BufReader::new(&file);
    //             let mut contents: Vec<u8> = Vec::new();
    //             reader.read_to_end(&mut contents);
    //             match String::from_utf8(contents.clone()) {
    //                 Ok(mut contents) if t != Typ::Bytes => {
    //                     if config.add_newlines && contents.ends_with('\n') {
    //                         contents.truncate(contents.len() - 1);
    //                     }
    //                     // TODO 2021-06-24 trim?
    //                     V::from_string(t, contents, &config)
    //                 }
    //                 Ok(_) | Err(_) => V::from_bytes(contents, &config),
    //             }
    //         }
    //         Err(_) => {
    //             match typ {
    //                 "map" => {
    //                     println!("map");
    //                 }
    //                 "list" => {
    //                     println!("list");
    //                 }
    //             }
    //         }
    //     }
    // }
    Ok()
}



fn main() -> std::io::Result<()> {
    let config = Config::from_pack_args();
    /* To pack
     * 1. get the path of the unpacked folder from config
     * 2. get the format from config
     * 3. traverse the unpacked folder and keep track of the path somehow
     * 4. for each file, get the xattr and convert that to Typ
     * 5. read the file into a string
     * 5. use Typ in the from_string function implemented for each format
     * 6. build the data structure (not sure how that will work yet)
     * 7. use to_writer to write to the output file given by config
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
            pack(folder, &config);
            // v.to_writer(writer, &config.pretty);
        }
        Format::Toml => {
            pack(folder, &config);
            // v.to_writer(writer, &config.pretty);
        }
        Format::Yaml => {
            pack(folder, &config);
            // v.to_writer(writer, &config.pretty);
        }
    }

    Ok(())
}
