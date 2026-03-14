use march_madness_common::EntryIndex;

fn main() {
    // Stub: will serve the entry index JSON over HTTP.
    let index = EntryIndex::new();
    println!(
        "march-madness-server: serving {} entries",
        index.len()
    );
}
