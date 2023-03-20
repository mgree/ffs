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
// use format::Format;
use format::Nodelike;
use format::Typ;
// use format::json::Value as JsonValue;
// use format::toml::Value as TomlValue;
use format::yaml::Value as YamlValue;

use ::xattr;

#[allow(dead_code)]
#[allow(unused_variables)]
fn pack<V>(root: V, root_path: PathBuf, config: &Config) -> std::io::Result<()>
where
    V: Nodelike + std::fmt::Display + Default,
{
    Ok(())
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
    let file = fs::File::open("invoice/invoice").unwrap();
    println!("file: {:?}", file);
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents).unwrap();
    println!("{}", contents);
    let typ = xattr::get("invoice/invoice", "user.type")?.unwrap();
    let typ = Typ::from_str(str::from_utf8(&typ).unwrap());
    let typ = typ.unwrap();
    println!("type: {:?}", typ);

    let val = YamlValue::from_string(typ, contents, &config);
    println!("val: {:?}", val);
    Ok(())
}
