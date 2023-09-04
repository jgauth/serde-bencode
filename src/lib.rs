mod de;
mod error;

// pub use crate::de::{from_str, Deserializer};
// pub use crate::error::{Error, Result};

// Here’s a small programming problem:
// write a function that takes a string of words separated by spaces and
// returns the first word it finds in that string.
// If the function doesn’t find a space in the string, the whole
// string must be one word, so the entire string should be returned.

// Let’s work through how we’d write the signature of this function
// without using slices, to understand the problem that slices will solve:

// fn first_word(s: &String) -> ?

fn first_word(s: &String) -> &str {
    match s.find(' ') {
        Some(i) => &s[..i],
        _ => &s[..],
    }
}
