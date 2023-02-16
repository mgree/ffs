use std::fs;
use std::env;
use std::io::Write;
use std::path::{PathBuf, Path};
use std::io::BufReader;
use std::collections::VecDeque;

use ffs::format;

use format::Nodelike;
use format::json::Value;

fn create_files(json: &Value, path: PathBuf) {
    let mut queue: VecDeque<(&Value, PathBuf)> = VecDeque::new();
    queue.push_back((json, path));

    while !queue.is_empty() {
        let (current,p) = queue.pop_front().unwrap();
        match current {
            Value::Null => {
                match fs::create_dir_all(p.parent().unwrap()) {
                    Ok(_) => (),
                    Err(e) => panic!("Error creating directory: {}", e),
                }
                let mut f: fs::File = match fs::OpenOptions::new().read(true).write(true).create_new(true).open(p) {
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
                let mut f: fs::File = match fs::OpenOptions::new().read(true).write(true).create_new(true).open(p) {
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
                let mut f: fs::File = match fs::OpenOptions::new().read(true).write(true).create_new(true).open(p) {
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
                let mut f: fs::File = match fs::OpenOptions::new().read(true).write(true).create_new(true).open(p) {
                    Ok(file) => file,
                    Err(e) => panic!("Error creating new file: {}", e),
                };
                match writeln!(f, "{}", s.to_string()) {
                    Ok(_) => (),
                    Err(e) => panic!("Error writing to file: {}", e),
                }
            }
            Value::Object(obj) => {
                for (k,v) in obj.iter() {
                    queue.push_back((v, p.join(k)));
                }
            }
            Value::Array(arr) => {
                for (i,v) in arr.iter().enumerate() {
                    queue.push_back((v, p.join(i.to_string())));
                }
            }
        }
    }
}

fn main() {
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
    create_files(&json_value, PathBuf::from(&cwd).join(&relative_file_path));

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
