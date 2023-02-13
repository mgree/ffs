
use std::fs;
use std::path::PathBuf;
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
                println!("Path {}", p.display());
            }
            Value::Bool(b) => {
                println!("Path {} Bool {}", p.display(), b.to_string());
            }
            Value::Number(n) => {
                println!("Path {} Number {}", p.display(), n.to_string());
            }
            Value::String(s) => {
                println!("Path {} String {}", p.display(), s);
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
    let filename = String::from("test.json");

    println!("filename: {}", filename);

    let file = fs::File::open("test.json").unwrap();
    let reader = Box::new(BufReader::new(file));
    let jsonvalue: Value = Nodelike::from_reader(reader);

    create_files(&jsonvalue, PathBuf::from("test"));

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
