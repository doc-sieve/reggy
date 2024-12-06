use std::fs::File;
use std::io::{self, BufReader};

use reggy::Search;

#[test]
fn stream() -> Result<(), io::Error> {
    let f = File::open("tests/samples/republic_plato.txt")?;
    let f = BufReader::new(f);

    let patterns = [
        r"yes|(very )?true|certainly|quite so|I have no objection|I agree",
        r"\?",
    ];

    let mut pattern_counts = [0; 2];

    let mut search = Search::compile(&patterns).unwrap();

    for result in search.iter(f) {
        match result {
            Ok(m) => {
                pattern_counts[m.id] += 1;
            }
            Err(e) => {
                println!("Stream Error {e:?}");
                break;
            }
        }
    }

    assert_eq!(pattern_counts[0], 1467);
    assert_eq!(pattern_counts[1], 1934);

    Ok(())
}
