use march_madness_common::EntryIndex;

fn main() {
    // Stub: will listen for BracketSubmitted events and maintain the index.
    let index = EntryIndex::new();
    println!(
        "march-madness-indexer: {} entries indexed",
        index.len()
    );
}
