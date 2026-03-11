use quickmatch::{QuickMatch, QuickMatchConfig};

fn main() {
    // Case 1: Items with colon, query with colon (default separators)
    let items = vec!["word: word2", "other: thing", "word word2"];
    let qm = QuickMatch::new(&items);
    let results = qm.matches("word: word2");
    println!("Default seps | query 'word: word2' vs items with colon: {results:?}");

    // Case 2: Query "word:" against items
    let items2 = vec!["word: word2", "word word2", "word", "word:"];
    let qm2 = QuickMatch::new(&items2);
    let results2 = qm2.matches("word:");
    println!("Default seps | query 'word:' vs mixed items:           {results2:?}");

    // Case 3: Colon as separator, query "word: word2"
    let config3 = QuickMatchConfig::new().with_separators(&['_', '-', ' ', ':']);
    let items3 = vec!["word: word2", "other: thing", "word word2"];
    let qm3 = QuickMatch::new_with(&items3, config3);
    let results3 = qm3.matches("word: word2");
    println!("Colon as sep | query 'word: word2':                    {results3:?}");

    // Case 4: Colon as separator, query "word:"
    let config4 = QuickMatchConfig::new().with_separators(&['_', '-', ' ', ':']);
    let items4 = vec!["word: word2", "word word2", "word", "word:"];
    let qm4 = QuickMatch::new_with(&items4, config4);
    let results4 = qm4.matches("word:");
    println!("Colon as sep | query 'word:' vs mixed items:           {results4:?}");
}
