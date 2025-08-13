use std::{ops::Deref, path::Path};

use triple_r::{ReusableHashMap, ReusableString};

fn read_file_and_count_words(
    path: impl AsRef<Path>,
    map: &mut ReusableHashMap<&str, usize>,
    buffer: &mut ReusableString,
) {
    use std::io::Read;
    let mut content = buffer.recycle();
    let mut map = map.recycle();
    let mut file = std::fs::File::open(path).unwrap();
    file.read_to_string(&mut content).unwrap();

    content.split_ascii_whitespace().for_each(|w| {
        *map.entry(w).or_default() += 1;
    });

    println!("{:#?}", map.deref())
}

fn main() {
    let mut counter = ReusableHashMap::<&str, usize>::default();
    let mut buffer = ReusableString::default();

    read_file_and_count_words("tests/random.txt", &mut counter, &mut buffer);
    read_file_and_count_words("tests/random.txt", &mut counter, &mut buffer);
    read_file_and_count_words("tests/random.txt", &mut counter, &mut buffer);
}
