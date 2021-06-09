use serde_json::Value;

use tracing::instrument;

#[instrument(level = "info", skip(reader))]
pub fn json(reader: Box<dyn std::io::BufRead>) -> Value {
    serde_json::from_reader(reader).expect("JSON")
}