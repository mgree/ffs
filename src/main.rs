use clap::{App, Arg};
use serde_json::Value;

fn main() {
    let config = App::new("ffs")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("file fileystem")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file (defaults to '-', meaning STDIN)")
                .default_value("-")
                .index(1),
        )
        .get_matches();

    let input_source = config.value_of("INPUT").unwrap();

    let reader: Box<dyn std::io::BufRead> = if input_source == "-" {
        Box::new(std::io::BufReader::new(std::io::stdin()))
    } else {
        let file = std::fs::File::open(input_source)
            .unwrap_or_else(|e| panic!("Unable to open {} for JSON input: {}", input_source, e));
        Box::new(std::io::BufReader::new(file))
    };

    let json: Value = serde_json::from_reader(reader).expect("JSON");

    println!("{:?}", json);
}
