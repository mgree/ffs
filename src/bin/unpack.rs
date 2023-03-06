use std::collections::VecDeque;
use std::env;
use std::fs;
use std::io::BufReader;
use std::io::Write;
use std::path::{Path, PathBuf};

use ffs::config::Config;
use ffs::format;

use format::json::Value;
use format::Nodelike;

#[allow(dead_code)]
fn unpack<V>(root: V, root_path: PathBuf, config: &Config) -> std::io::Result<()>
where
    V: Nodelike + std::fmt::Display + Default,
{
    let mut queue: VecDeque<(V, PathBuf, Option<String>)> = VecDeque::new();
    queue.push_back((root, root_path, None));

    while !queue.is_empty() {
        let (v, path, original_name) = queue.pop_front().unwrap();

        match v.node(config) {
            format::Node::String(_t, s) => {
                // make a regular file at `path`
                let mut f = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true) // TODO(mmg) 2023-03-06 allow truncation?
                    .open(path)?;

                // write `s` into that file
                write!(f, "{}", s)?;

                // set metadata according to `t`
                // TODO(mmg) 2023-03-06 set `user.type` metadata using setxattr
            }
            format::Node::Bytes(b) => {
                // make a regular file at `path`
                let mut f = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true) // TODO(mmg) 2023-03-06 allow truncation?
                    .open(path)?;

                // write `b` into that file
                f.write_all(b.as_slice())?;

                // set metadata to bytes
                // TODO(mmg) 2023-03-06 set `user.type` metadata using setxattr
            }
            format::Node::List(vs) => {
                // make directory
                fs::create_dir(&path)?;
                // TODO(mmg) 2023-03-06 set directory metadata to list using setxattr

                // enqueue children with appropriate names
                let num_elts = vs.len() as f64;
                let width = num_elts.log10().ceil() as usize;

                for (i, child) in vs.into_iter().enumerate() {
                    // TODO(mmg) 2021-06-08 ability to add prefixes
                    let name = if config.pad_element_names {
                        format!("{:0width$}", i, width = width)
                    } else {
                        format!("{}", i)
                    };
                    let child_path = path.join(name);

                    queue.push_back((child, child_path, None));
                }
            }
            format::Node::Map(fvs) => {
                // make directory
                fs::create_dir(&path)?;
                // TODO(mmg) 2023-03-06 set directory metadata to map using setxattr

                // enqueue children with appropriate names
                let mut child_names = std::collections::HashSet::new();
                for (field, child) in fvs.into_iter() {
                    let original = field.clone();

                    // munge name to be valid and uniqueÍ
                    let name = if !config.valid_name(&original) {
                        match config.munge {
                            ffs::config::Munge::Rename => {
                                let mut nfield = config.normalize_name(field);

                                while child_names.contains(&nfield) {
                                    nfield.push('_');
                                }

                                nfield
                            }
                            ffs::config::Munge::Filter => {
                                // TODO(mmg) 2023-03-06 support logging
                                //warn!("skipping '{}'", field);
                                continue;
                            }
                        }
                    } else {
                        field
                    };
                    child_names.insert(name.clone());

                    let child_path = path.join(name);
                    queue.push_back((child, child_path, Some(original)));
                }
            }
        }

        if let Some(_original_name) = original_name {
            // TODO(mmg) 2023-03-6 set `user.original_name` using setxattr
        }
    }

    Ok(())
}

#[allow(dead_code)]
fn create_files(json: &Value, path: PathBuf) {
    let mut queue: VecDeque<(&Value, PathBuf)> = VecDeque::new();
    queue.push_back((json, path));

    while !queue.is_empty() {
        let (current, p) = queue.pop_front().unwrap();
        match current {
            Value::Null => {
                match fs::create_dir_all(p.parent().unwrap()) {
                    Ok(_) => (),
                    Err(e) => panic!("Error creating directory: {}", e),
                }
                let mut f: fs::File = match fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .open(p)
                {
                    Ok(file) => file,
                    Err(e) => panic!("Error creating new file: {}", e),
                };
                match writeln!(f, "") {
                    Ok(_) => (),
                    Err(e) => panic!("Error writing to file: {}", e),
                }
            }
            Value::Bool(b) => {
                match fs::create_dir_all(p.parent().unwrap()) {
                    Ok(_) => (),
                    Err(e) => panic!("Error creating directory: {}", e),
                }
                let mut f: fs::File = match fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .open(p)
                {
                    Ok(file) => file,
                    Err(e) => panic!("Error creating new file: {}", e),
                };
                match writeln!(f, "{}", b.to_string()) {
                    Ok(_) => (),
                    Err(e) => panic!("Error writing to file: {}", e),
                }
            }
            Value::Number(n) => {
                match fs::create_dir_all(p.parent().unwrap()) {
                    Ok(_) => (),
                    Err(e) => panic!("Error creating directory: {}", e),
                }
                let mut f: fs::File = match fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .open(p)
                {
                    Ok(file) => file,
                    Err(e) => panic!("Error creating new file: {}", e),
                };
                match writeln!(f, "{}", n.to_string()) {
                    Ok(_) => (),
                    Err(e) => panic!("Error writing to file: {}", e),
                }
            }
            Value::String(s) => {
                // println!("Path {} String {}", p.display(), s);
                match fs::create_dir_all(p.parent().unwrap()) {
                    Ok(_) => (),
                    Err(e) => panic!("Error creating directory: {}", e),
                }
                let mut f: fs::File = match fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .open(p)
                {
                    Ok(file) => file,
                    Err(e) => panic!("Error creating new file: {}", e),
                };
                match writeln!(f, "{}", s.to_string()) {
                    Ok(_) => (),
                    Err(e) => panic!("Error writing to file: {}", e),
                }
            }
            Value::Object(obj) => {
                for (k, v) in obj.iter() {
                    queue.push_back((v, p.join(k)));
                }
            }
            Value::Array(arr) => {
                for (i, v) in arr.iter().enumerate() {
                    queue.push_back((v, p.join(i.to_string())));
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<_> = env::args().collect();
    let filename = String::from(&args[1]);

    let cwd = env::current_dir().unwrap();

    // println!("filename: {}", filename);

    let file = fs::File::open(&filename).unwrap();
    let reader = Box::new(BufReader::new(file));
    let json_value: Value = Nodelike::from_reader(reader);

    let relative_file_path = Path::new(&filename).file_stem().unwrap().to_str().unwrap();

    if Path::new(&relative_file_path).exists() {
        panic!("Directory {} already exists", relative_file_path);
    }
    // TODO add subdirectory check not just root directory check
    //    create_files(&json_value, PathBuf::from(&cwd).join(&relative_file_path));

    unpack(
        json_value,
        PathBuf::from(&cwd).join(&relative_file_path),
        &Config::default(),
    )

    /*
    - get json file from options
    - get other options
    - parse json file using serde
    - use fn to (recursively) or iteratively create directories.
        - if directory exists, maybe raise error (given command line options)
        - if directory does not exist, create it
        - maybe use iterative bfs with a queue of json values.
            - probably not as memory efficient in terms of storing multiple copies of the same thing?
            - if it is not a json array or object create a file.
            - if it is a json array or object, create a directory and add references of the sub objects to the queue.
    */
}
