/*!
These are a pair of simple structures for monitoring a stream of data looking for a specific
sequence of values.

The original purpose for this crate was to monitor bytes of data recieved from stdin for a specific
sequence that indicated that the input thread should shut down. Originally, only a single sequence
was monitored for, but I added a simple way to look for multiple sequences simultaneously.


If you want multiple seperate sequences simultaneously, you can use a [SequenceWatchers] structure.

*/
use std::{
    fmt::Debug,
};

/**
Monitors a stream of data for multiple sequences of data items.

```
use seq_watcher::SequenceWatchers;

const CTRL_C: u8 = 3;
const CTRL_D: u8 = 4;

// Watch the input stream for two consecutive <CTRL-C> characters.
let mut quit_watchers = SequenceWatchers::new(&[&[CTRL_C, CTRL_C], &[CTRL_C, CTRL_D]]);


for b in b'a'..=b'z' {
    assert_eq!(false, quit_watchers.check(&b));   // Send single ASCII byte
    assert_eq!(false, quit_watchers.check(&CTRL_C));   // Send single <Ctrl-C>
}
assert_eq!(true, quit_watchers.check(&CTRL_C));    // Send a second <Ctrl-C>
assert_eq!(true, quit_watchers.check(&CTRL_D));    // Send a <Ctrl-D>
```
*/
#[derive(Debug)]
pub struct SequenceWatchers<T>
where
    T: PartialEq + Clone + Debug,
{
    watchers: Vec<SequenceWatcher<T>>,
}

/**
*/
impl<T: PartialEq + Clone + Debug> SequenceWatchers<T> {
    /**
    Create a new [SequenceWatchers] from a slice of slices of data. The data type can be anything
    with [PartialEq], [Debug], and [Clone] traits.

    Internally, the [SequenceWatchers] structure contains a vector of [SequenceWatcher] structures.

    ## Example
    ```
    use seq_watcher::SequenceWatchers;

    let mut quit_watchers = SequenceWatchers::new(&[&['q'], &['Q']]);

    assert_eq!(true, quit_watchers.check(&'q'));
    assert_eq!(true, quit_watchers.check(&'Q'));
    ```
    */
    pub fn new(sequences: &[&[T]]) -> Self {
        let mut watchers = Vec::with_capacity(sequences.len());
        for seq in sequences {
            watchers.push(SequenceWatcher::new(seq));
        }
        Self { watchers }
    }

    /**
    Tells the [SequenceWatchers] of a new input item and checks to see if a sequence has been
    completed. Returns true when a sequence is completed.

    ## Example

    ```
    use seq_watcher::SequenceWatchers;

    let mut quit_watchers = SequenceWatchers::new(&[&[false, true], &[false, true, true]]);

    assert_eq!(false, quit_watchers.check(&true));
    assert_eq!(false, quit_watchers.check(&false));
    assert_eq!(true, quit_watchers.check(&true));   // Matches first sequence
    assert_eq!(true, quit_watchers.check(&true));   // Matches second sequence
    ```
    */
    pub fn check(&mut self, value: &T) -> bool {
        let mut result = false;
        for watcher in &mut self.watchers {
            if watcher.check(value) {
                result = true;
            }
        }
        result
    }
}

/**
Monitors a stream of data for a sequence.

```
use seq_watcher::SequenceWatcher;

const CTRL_C: u8 = 3;

// Watch the input stream for two consecutive <CTRL-C> characters.
let mut quit_watcher = SequenceWatcher::new(&[CTRL_C, CTRL_C]);


for b in b'a'..=b'z' {
    assert_eq!(false, quit_watcher.check(&b));   // Send single ASCII byte
    assert_eq!(false, quit_watcher.check(&CTRL_C));   // Send single <Ctrl-C>
}
assert_eq!(true, quit_watcher.check(&CTRL_C));    // Send a second <Ctrl-C>
```
*/
#[derive(Clone, Debug)]
pub struct SequenceWatcher<T>
where
    T: PartialEq + Clone + Debug,
{
    sequence: Vec<T>,
    index: usize,
}

impl<T: PartialEq + Clone + Debug> SequenceWatcher<T> {
    /**
    Create a new [SequenceWatcher] from a slice of data. The data type can be anything with
    [PartialEq], [Debug], and [Clone] traits.

    Internally, the [SequenceWatcher] structure contains a vector of the sequence it monitors and
    an index indicating where in the sequence it is expecting next.

    ## Example
    ```
    use seq_watcher::SequenceWatcher;

    let mut quit_watcher = SequenceWatcher::new(&['q', 'u', 'i', 't']);

    assert_eq!(false, quit_watcher.check(&'q'));
    assert_eq!(false, quit_watcher.check(&'u'));
    assert_eq!(false, quit_watcher.check(&'i'));
    assert_eq!(true, quit_watcher.check(&'t'));
    ```
    */
    pub fn new(seq: &[T]) -> Self {
        Self {
            sequence: seq.to_vec(),
            index: 0,
        }
    }

    /**
    Tells the [SequenceWatcher] of a new input item and checks to see if a sequence has been
    completed. Returns true when the sequence is completed.

    ## Example

    ```
    use seq_watcher::SequenceWatcher;

    let mut quit_watcher = SequenceWatcher::new(&[Some("quit")]);

    assert_eq!(false, quit_watcher.check(&None));
    assert_eq!(false, quit_watcher.check(&Some("something")));
    assert_eq!(false, quit_watcher.check(&Some("anything")));
    assert_eq!(true, quit_watcher.check(&Some("quit")));
    ```
    */
    pub fn check(&mut self, value: &T) -> bool {
        if self.sequence.is_empty() { return false; }
        if *value == self.sequence[self.index] {
            self.index += 1;
            if self.sequence.len() > self.index {
                false
            }
            else {
                self.index = 0;
                true
            }
        }
        else {
            match self.index {
                0 => false,
                1 => {
                    // If we only matched one previous value, just reset the index and try the new
                    // value again.
                    self.index = 0;
                    self.check(value)
                },
                _ => {
                    // If we have a mismatch after a partial match, we need to make sure that we
                    // don't lose any previous characters.
                    let old_index = self.index;
                    self.index = 0;
                    let mut new_start = None;
                    let mut i = 1;
                    while i < old_index {
                        if self.sequence[i] == self.sequence[self.index] {
                            self.index += 1;
                            if None == new_start {
                                new_start = Some(i)
                            }
                        }
                        else {
                            // If we get a mismatch, did we already have a partial match?
                            if let Some(j) = new_start {
                                // On a previous partial match restart the matching at the index
                                // after the previous start.
                                i = j;  // Will be incremented before the next test.
                                self.index = 0;
                                new_start = None;
                            }
                        }
                        i += 1;
                    }
                    self.check(value)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fmt::Debug,
    };
    use proptest::{
        prelude::*,
        sample::SizeRange,
    };
    use crate::SequenceWatcher;

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum ByteOrChar {
        Byte(u8),
        Char(char),
    }

    impl From<u8> for ByteOrChar {
        fn from(src: u8) -> Self {
            ByteOrChar::Byte(src)
        }
    }

    impl From<char> for ByteOrChar {
        fn from(src: char) -> Self {
            if 1 == src.len_utf8() {
                ByteOrChar::Byte(src as u8)
            }
            else {
                ByteOrChar::Char(src)
            }
        }
    }

    // Takes a sequence vector and a stream vector and returns the sequence vector and a
    // stream vector that is guaranteed to succeed on the last value.
    //
    // It verifies that there are no instances of the sequence vector in the stream vector, and
    // then appends the sequence vector to the stream vector and returns the original sequence
    // vector and an updated sequence vector.
    fn values_and_stream<T: PartialEq + Clone + Debug>(sequence: Vec<T>, mut stream: Vec<T>) -> (Vec<T>, Vec<T>) {
        let seq_len = sequence.len();
        let mut i = 0;
        stream.extend_from_slice(&sequence);
        while (i + seq_len) < stream.len() {
            if &sequence[..] == &stream[i..(i+seq_len)] {
                if i + seq_len + seq_len <= stream.len() {
                    // If we are not to the final sequence, remove the last element.
                    stream.remove(i+seq_len-1);
                }
                else {
                    // If we are overlapping the final sequence, then remove the first value.
                    stream.remove(i);
                    // Since we remove the first value, we may inadvertently be creating a valid
                    // sequence with the previous value, so we need to recheck the index before.
                    if 0 < i { i -= 1; }    // If the current index is not 0.
                }
            }
            else {
                // Otherwise, move to the next sub-vector.
                i += 1;
            }
        }
        // Now, add the correct sequence to the end so that it will always succed on the last byte.
        (sequence, stream)
    }

    // This makes sure that the values_and_stream function always returns the sequence only at the
    // end.
    #[test]
    fn test_values_and_stream_test_function() {
        {
            let (find, stream) = values_and_stream(vec![b'a', b'a'], vec![b'a']);
            test_seq_watcher(&find, &stream).unwrap();
        }
        {
            let (find, stream) = values_and_stream(vec![b'a', b'b', b'a'], vec![b'b', b'a', b'b', b'a']);
            test_seq_watcher(&find, &stream).unwrap();
        }
    }

    // Takes a sequence slice and a stream slice and verifies that a Sequence Watcher with the
    // sequence only succeeds on the last value in the stream slice.
    fn test_seq_watcher<T: PartialEq + Clone + Debug>(sequence: &[T], stream: &[T]) -> Result<(), proptest::test_runner::TestCaseError> {
        prop_assert!(0 < stream.len(), "Invalid stream: {:?}", stream);
        prop_assert!(0 < sequence.len(), "Invalid sequence: {:?}", sequence);
        let mut seq_watcher = SequenceWatcher::new(sequence);
        let last_index = stream.len() - 1;
        for i in 0..last_index {
            prop_assert_eq!(false, seq_watcher.check(&stream[i]));
        }
        prop_assert_eq!(true, seq_watcher.check(&stream[last_index]));
        Ok(())
    }

    prop_compose! {
        fn byte_vec(range: impl Into<SizeRange>)(v in prop::collection::vec(0..u8::MAX, range)) -> Vec<u8> { v }
    }

    prop_compose! {
        fn char_vec(range: impl Into<SizeRange>)(v in prop::collection::vec(proptest::char::any(), range)) -> Vec<char> { v }
    }

    prop_compose! {
        fn byte_or_char_vec(range: impl Into<SizeRange>)(v in char_vec(range)) -> Vec<ByteOrChar> {
            let v: Vec<ByteOrChar> = v.into_iter().map(|bc| {ByteOrChar::from(bc)}).collect();
            v
        }
    }

    prop_compose! {
        fn bool_vec(range: impl Into<SizeRange>)(v in prop::collection::vec(proptest::bool::ANY, range)) -> Vec<bool> { v }
    }

    prop_compose! {
        fn chars_and_stream()(seq in char_vec(1..10), v in char_vec(10..50)) -> (Vec<char>, Vec<char>)
        {
            values_and_stream(seq, v)
        }
    }

    prop_compose! {
        fn bytes_and_stream()(seq in byte_vec(1..10), v in byte_vec(10..50)) -> (Vec<u8>, Vec<u8>)
        {
            values_and_stream(seq, v)
        }
    }

    prop_compose! {
        fn byte_or_chars_and_stream()(seq in byte_or_char_vec(1..10), v in byte_or_char_vec(10..50)) -> (Vec<ByteOrChar>, Vec<ByteOrChar>)
        {
            values_and_stream(seq, v)
        }
    }

    prop_compose! {
        fn bools_and_stream()(seq in bool_vec(1..10), v in bool_vec(10..50)) -> (Vec<bool>, Vec<bool>)
        {
            values_and_stream(seq, v)
        }
    }

    proptest! {
        #[test]
        fn test_chars_sequence((find, stream) in chars_and_stream()) {
            test_seq_watcher(&find[..], &stream[..])?;
        }
    }

    proptest! {
        #[test]
        fn test_bytes_sequence((find, stream) in bytes_and_stream()) {
            test_seq_watcher(&find[..], &stream[..])?;
        }
    }

    proptest! {
        #[test]
        fn test_byte_or_chars_sequence((find, stream) in byte_or_chars_and_stream()) {
            test_seq_watcher(&find[..], &stream[..])?;
        }
    }

    proptest! {
        #[test]
        fn test_bools_sequence((find, stream) in bools_and_stream()) {
            test_seq_watcher(&find[..], &stream[..])?;
        }
    }

    proptest! {
        #[test]
        fn empty_char_sequence_is_always_false(stream in char_vec(10..50)) {
            let mut seq_watcher = SequenceWatcher::new(&[]);
            for c in stream {
                prop_assert_eq!(false, seq_watcher.check(&c));
            }
        }
    }

    proptest! {
        #[test]
        fn empty_byte_sequence_is_always_false(stream in byte_vec(10..50)) {
            let mut seq_watcher = SequenceWatcher::new(&[]);
            for b in stream {
                prop_assert_eq!(false, seq_watcher.check(&b));
            }
        }
    }

    proptest! {
        #[test]
        fn empty_byte_or_char_sequence_is_always_false(stream in byte_or_char_vec(10..50)) {
            let mut seq_watcher = SequenceWatcher::new(&[]);
            for c in stream {
                prop_assert_eq!(false, seq_watcher.check(&c));
            }
        }
    }

    #[test]
    fn test_specific_patterns() {   // And also some different types.
        // This will fail if we just discard the state on a mismatch. On a failing case, when it
        // hits the second zero in the stream, it forgets about both zeroes and then doesn't match
        // the 1 because it is expecting a zero first.
        test_seq_watcher(&[false, true], &[false, false, true]).unwrap();
        // Let's say that we are smarter and pass the first test by retrying the current character
        // after resetting the index. what happens if we need a partial reset.
        test_seq_watcher(&[0u64, 0, 1], &[0, 0, 0, 1]).unwrap();
        // Now we need a pattern that will cause the simple fix for the second test to fail (or
        // maybe I don't need the more complicated solution). 
        test_seq_watcher(&["F", "T", "F", "F", "F"], &["F", "T", "F", "F", "T", "F", "F", "F"]).unwrap();
        // Now we'll just repeat the last pattern with some alternate types.
        test_seq_watcher(&[None, Some(0.0), None, None, None], &[None, Some(0.0), None, None, Some(0.0), None, None, None]).unwrap();
        test_seq_watcher(&[Err(()), Ok(()), Err(()), Err(()), Err(())], &[Err(()), Ok(()), Err(()), Err(()), Ok(()), Err(()), Err(()), Err(())]).unwrap();
    }
}
