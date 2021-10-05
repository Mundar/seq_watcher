//! # Stream Sequence Watcher
//!
//! Includes a pair of structures, [SequenceWatcher] and [SequenceWatchers]. They monitor a stream
//! of data looking for a specific sequence of values.
//!
//! The original purpose for this crate was to monitor bytes of data recieved from stdin for a
//! specific sequence that indicated that the input thread should shut down. Originally, only a
//! single sequence was monitored for, but I added a simple way to look for multiple sequences
//! simultaneously.
//!
//! # Examples
//!
//! ## Intended Behavior and Limitations
//!
//! * [SequenceWatcher] and [SequenceWatchers] both continue monitoring the stream after a check
//!   returns true. For example, looking for the sequence (A, A), would return false, true, true
//!   when given the input (A, A, A).
//!
//! ```
//! use seq_watcher::SequenceWatchers;
//!
//! let test_stream = vec![
//!     ('A', false),
//!     ('A', false),
//!     ('A', true),    // Matches AAA
//!     ('A', true),    // Matches AAA
//!     ('B', false),
//!     ('A', true),    // Matches ABA
//!     ('B', false),
//!     ('A', true),    // Matches ABA
//!     ('A', false),
//!     ('A', true),    // Matches AAA
//!     ('C', true),    // Matches C
//! ];
//!
//! let watchers = SequenceWatchers::new(&[&['A', 'A', 'A'], &['A', 'B', 'A'], &['C']]);
//!
//! for (ch, expect) in test_stream {
//!     assert_eq!(watchers.check(&ch), expect);
//! }
//!
//! ```
//!
//! * You can't create a [SequenceWatcher] with no sequence. [`SequenceWatcher::new`] will panic if
//!   given an empty slice.
//!
//! ```should_panic
//! # use seq_watcher::SequenceWatcher;
//! let watcher: SequenceWatcher<u8> = SequenceWatcher::new(&[]);
//! ```
//!
//! If you use [From] with an array reference, it will catch the empty array case at runtime.
//!
//! ```compile_fail
//! # use seq_watcher::SequenceWatcher;
//! let watcher: SequenceWatcher<u8> = SequenceWatcher::from(&[]);
//! ```
//!
//! However, [SequenceWatchers] handles empty slices fine. If there are no sequences, it will
//! always return false.
//!
//! ```
//! # use seq_watcher::SequenceWatchers;
//! let mut watchers = SequenceWatchers::new(&[]);  // Create empty watchers.
//! let mut all_bytes = Vec::with_capacity(256);
//! for b in 0..=u8::MAX {
//!     assert_eq!(watchers.check(&b), false);  // With no sequences, all checks are false.
//!     all_bytes.push(b);                      // Generate sequence with all bytes.
//! }
//! watchers.add(&[]);          // Add of empty sequence does nothing.
//! watchers.add(&all_bytes);   // Add sequuence with all bytes.
//! for b in 0..u8::MAX {
//!     assert_eq!(watchers.check(&b), false);   // Until sequence recieves the last byte, false.
//! }
//! assert_eq!(watchers.check(&u8::MAX), true); // With last byte in sequence, returns true.
//! ```
//!
//! * Datatypes compatible are slightly more restrictive for [SequenceWatchers] than for
//!   [SequenceWatcher]. [SequenceWatchers] requires the datatype to be [Eq], wheras
//!   [SequenceWatcher] only needs [PartialEq].
//!
//! Float types are [PartialEq] but not [Eq].
//!
//! ```compile_fail
//! # use seq_watcher::SequenceWatchers;
//! let watchers = SequenceWatchers::new(&[0.0]);   // Float values are not Eq
//! ```
//!
//! ```
//! # use seq_watcher::SequenceWatcher;
//! let watcher = SequenceWatcher::new(&[0.0]);     // Float values are PartialEq.
//! ```
//!
//! # Performance
//!
//! The [SequenceWatcher] structure is resonably performant, but the [SequenceWatchers] structure
//! needs work. [SequenceWatchers] is currently implemented as a vector of [SequenceWatcher]
//! structures, but it would be better implemented with some sort of multi-state-aware trie.
//! [SequenceWatchers] was created as an afterthought, since I mainly needed the [SequenceWatcher]
//! for my other project.
use std::{
    cell::Cell,
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
};

/// Monitors a stream of data for multiple sequences of data items.
///
/// # Examples
///
/// ```
/// use seq_watcher::SequenceWatchers;
///
/// const CTRL_C: u8 = 3;
/// const CTRL_D: u8 = 4;
///
/// // Watch the input stream for two consecutive <CTRL-C> characters.
/// let watchers = SequenceWatchers::new(&[&[CTRL_C, CTRL_C], &[CTRL_C, CTRL_D]]);
///
/// for b in b'a'..=b'z' {
///     assert_eq!(false, watchers.check(&b));   // Send single ASCII byte
///     assert_eq!(false, watchers.check(&CTRL_C));   // Send single <Ctrl-C>
/// }
/// assert_eq!(true, watchers.check(&CTRL_C));    // Send a second <Ctrl-C>
/// assert_eq!(true, watchers.check(&CTRL_D));    // Send a <Ctrl-D>
/// ```
#[derive(Debug)]
pub struct SequenceWatchers<T>
where
    T: Eq + Clone + Debug + Hash,
{
    watchers: HashMap<Vec<T>, SequenceWatcher<T>>,
}

impl<T: Eq + Clone + Debug + Hash> SequenceWatchers<T> {
    /// Create a new [SequenceWatchers] from a slice of slices of data. The data type can be
    /// anything with [Eq], [Debug], and [Clone] traits.
    ///
    /// Internally, the [SequenceWatchers] structure contains a vector of [SequenceWatcher]
    /// structures.
    ///
    /// # Examples
    ///
    /// ```
    /// # use seq_watcher::SequenceWatchers;
    /// let watchers = SequenceWatchers::new(&[&['q'], &['Q']]);
    ///
    /// assert_eq!(true, watchers.check(&'q'));
    /// assert_eq!(true, watchers.check(&'Q'));
    /// ```
    pub fn new(sequences: &[&[T]]) -> Self {
        let mut watchers = Self { watchers: HashMap::with_capacity(sequences.len()) };
        for seq in sequences {
            watchers.add(seq);
        }
        watchers
    }

    /// Tells the [SequenceWatchers] of a new input item and checks to see if a sequence has been
    /// completed. Returns true when a sequence is completed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use seq_watcher::SequenceWatchers;
    /// let watchers = SequenceWatchers::new(&[&[false, true], &[false, true, true]]);
    ///
    /// assert_eq!(false, watchers.check(&true));
    /// assert_eq!(false, watchers.check(&false));
    /// assert_eq!(true, watchers.check(&true));    // Matches first sequence
    /// assert_eq!(true, watchers.check(&true));    // Matches second sequence
    /// ```
    pub fn check(&self, value: &T) -> bool {
        let mut result = false;
        for watcher in self.watchers.values() {
            if watcher.check(value) {
                result = true;
            }
        }
        result
    }

    /// Adds a new [SequenceWatcher] to the [SequenceWatchers]. If the sequence matches a
    /// pre-existing [SequenceWatcher], it does nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// # use seq_watcher::SequenceWatchers;
    /// let mut watchers = SequenceWatchers::new(&[]);
    /// watchers.add(&[]);  // This does nothing.
    /// watchers.add(&[false, true]);
    /// watchers.add(&[false, true, true]);
    ///
    /// assert_eq!(false, watchers.check(&true));
    /// assert_eq!(false, watchers.check(&false));
    /// assert_eq!(true, watchers.check(&true));    // Matches first sequence
    /// assert_eq!(true, watchers.check(&true));    // Matches second sequence
    /// ```
    pub fn add(&mut self, new_seq: &[T]) {
        if !new_seq.is_empty() && !self.watchers.contains_key(new_seq) {
            self.watchers.insert(new_seq.to_vec(), SequenceWatcher::new(new_seq));
        }
    }

    /// Removes a sequence watcher for a given sequence. It returns the removed [SequenceWatcher].
    ///
    /// # Examples
    ///
    /// ```
    /// # use seq_watcher::SequenceWatchers;
    /// let mut watchers = SequenceWatchers::new(&[&[false, true], &[false, true, true]]);
    /// watchers.remove(&[false, true, true]);
    ///
    /// assert_eq!(false, watchers.check(&true));
    /// assert_eq!(false, watchers.check(&false));
    /// assert_eq!(true, watchers.check(&true));    // Matches first sequence
    /// assert_eq!(false, watchers.check(&true));   // Matches second sequence, which was removed
    /// ```
    pub fn remove(&mut self, seq: &[T]) -> Option<SequenceWatcher<T>> {
        self.watchers.remove(seq)
    }

    /// Returns the number of sequences being monitored.
    ///
    /// # Examples
    ///
    /// ```
    /// # use seq_watcher::SequenceWatchers;
    /// let watchers = SequenceWatchers::new(&[&['A'], &['B'], &['C'], &['D']]);
    ///
    /// assert_eq!(4, watchers.len());
    /// ```
    pub fn len(&self) -> usize {
        self.watchers.len()
    }

    /// Returns an iterator over the sequences being monitored by the [SequenceWatchers].
    ///
    /// # Examples
    ///
    /// ```
    /// # use seq_watcher::SequenceWatchers;
    /// let watchers = SequenceWatchers::new(&[&[false, true], &[false, true, true]]);
    ///
    /// let mut sequences: Vec<&Vec<bool>> = watchers.sequences().collect();
    /// sequences.sort();   // Make sure keys are sorted.
    /// let mut sequences = sequences.into_iter();
    /// assert_eq!(sequences.next(), Some(&vec![false, true]));
    /// assert_eq!(sequences.next(), Some(&vec![false, true, true]));
    /// assert_eq!(sequences.next(), None);
    /// ```
    pub fn sequences(&self) -> std::collections::hash_map::Keys<Vec<T>, SequenceWatcher<T>> {
        self.watchers.keys()
    }
}

/// Monitors a stream of data for a sequence.
///
/// # Examples
///
/// ```
/// use seq_watcher::SequenceWatcher;
///
/// const CTRL_C: u8 = 3;
///
/// // Watch the input stream for two consecutive <CTRL-C> characters.
/// let watcher = SequenceWatcher::new(&[CTRL_C, CTRL_C]);
///
/// for b in b'a'..=b'z' {
///     assert_eq!(false, watcher.check(&b));   // Send single ASCII byte
///     assert_eq!(false, watcher.check(&CTRL_C));   // Send single <Ctrl-C>
/// }
/// assert_eq!(true, watcher.check(&CTRL_C));    // Send a second <Ctrl-C>
/// ```
#[derive(Clone, Debug)]
pub struct SequenceWatcher<T>
where
    T: PartialEq + Clone + Debug,
{
    sequence: Vec<T>,
    index: Cell<usize>,
}

impl<T: PartialEq + Clone + Debug> SequenceWatcher<T> {
    /// Create a new [SequenceWatcher] from a slice of data. The data type can be anything with
    /// [PartialEq], [Debug], and [Clone] traits.
    ///
    /// Internally, the [SequenceWatcher] structure contains a vector of the sequence it monitors
    /// and an index indicating where in the sequence it is expecting next.
    ///
    /// # Examples
    ///
    /// ```
    /// # use seq_watcher::SequenceWatcher;
    /// let watcher = SequenceWatcher::new(&['q', 'u', 'i', 't']);
    ///
    /// assert_eq!(false, watcher.check(&'q'));
    /// assert_eq!(false, watcher.check(&'u'));
    /// assert_eq!(false, watcher.check(&'i'));
    /// assert_eq!(true, watcher.check(&'t'));
    /// ```
    ///
    /// # Panics
    ///
    /// [`SequenceWatcher::new`] will panic if the sequence slice is empty.
    pub fn new(seq: &[T]) -> Self {
        assert!(!seq.is_empty(), "Error: Can't create a SequenceWatcher with no sequence!");
        Self {
            sequence: seq.to_vec(),
            index: Cell::new(0),
        }
    }

    /// Tells the [SequenceWatcher] of a new input item and checks to see if a sequence has been
    /// completed. Returns true when the sequence is completed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use seq_watcher::SequenceWatcher;
    /// let watcher = SequenceWatcher::new(&[Some("found")]);
    ///
    /// assert_eq!(false, watcher.check(&None));
    /// assert_eq!(false, watcher.check(&Some("something")));
    /// assert_eq!(false, watcher.check(&Some("anything")));
    /// assert_eq!(true, watcher.check(&Some("found")));
    /// ```
    pub fn check(&self, value: &T) -> bool {
        self.witness(value);
        if self.sequence.len() == self.index.get() {
            self.on_mismatch(&self.sequence[1..]);
            true
        }
        else {
            false
        }
    }

    /// Returns the a reference to the data value that the [SequenceWatcher] is expecting next.
    ///
    /// # Examples
    ///
    /// ```
    /// # use seq_watcher::SequenceWatcher;
    /// let watcher = SequenceWatcher::new(&[1, 2]);
    ///
    /// assert_eq!(1, *watcher.expects());
    /// assert_eq!(false, watcher.check(&0));
    /// assert_eq!(1, *watcher.expects());
    /// assert_eq!(false, watcher.check(&1));
    /// assert_eq!(2, *watcher.expects());
    /// assert_eq!(true, watcher.check(&2));
    /// assert_eq!(1, *watcher.expects());
    /// ```
    pub fn expects(&self) -> &T {
        &self.sequence[self.index.get()]
    }

    /// Returns the index in the sequence that the [SequenceWatcher] is expecting next. The index
    /// will never be greater than or equal to the sequence length.
    ///
    /// # Examples
    ///
    /// ```
    /// # use seq_watcher::SequenceWatcher;
    /// let watcher = SequenceWatcher::new(&[1, 2]);
    ///
    /// assert_eq!(0, watcher.index());
    /// assert_eq!(false, watcher.check(&0));
    /// assert_eq!(0, watcher.index());
    /// assert_eq!(false, watcher.check(&1));
    /// assert_eq!(1, watcher.index());
    /// assert_eq!(true, watcher.check(&2));
    /// assert_eq!(0, watcher.index());
    /// ```
    #[inline]
    pub fn index(&self) -> usize {
        self.index.get()
    }

    /// Resets the [SequenceWatcher] so it doesn't remember if it has seen any protions of the
    /// sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// # use seq_watcher::SequenceWatcher;
    /// let watcher = SequenceWatcher::new(&[1, 2]);
    ///
    /// assert_eq!(false, watcher.check(&1));
    /// assert_eq!(1, watcher.index());
    /// watcher.reset();
    /// assert_eq!(false, watcher.check(&2));
    /// assert_eq!(false, watcher.check(&1));
    /// assert_eq!(true, watcher.check(&2));
    /// ```
    pub fn reset(&self) {
        self.index.set(0);
    }

    /// Returns a reference to the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// # use seq_watcher::SequenceWatcher;
    /// let watcher = SequenceWatcher::new(&[Ok(()), Err(())]);
    ///
    /// assert_eq!(&[Ok(()), Err(())], watcher.sequence());
    /// ```
    pub fn sequence(&self) -> &[T] {
        &self.sequence[..]
    }

    /// Returns the length of the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// # use seq_watcher::SequenceWatcher;
    /// let watcher = SequenceWatcher::new(&[false, true, false, true]);
    ///
    /// assert_eq!(4, watcher.len());
    /// ```
    pub fn len(&self) -> usize {
        self.sequence.len()
    }

    // Internal function that does all of the work of monitoring a new data type, but doesn't check
    // to see if it has successfully reached the end of the sequence. This shouldn't be a public
    // interface because it can leave the watcher in an invalid state (i.e. watchers should always
    // be able to accept new input data, but this can leave the index pointing past the end of the
    // vector.
    fn witness(&self, value: &T) {
        if self.sequence.is_empty() { return; }
        if *value == self.sequence[self.index.get()] {
            self.index.set(self.index.get() + 1);
        }
        else {
            match self.index.get() {
                0 => {},
                1 => {
                    self.index.set(0);
                    self.witness(value)
                },
                _ => {
                    self.on_mismatch(&self.sequence[1..self.index.get()]);
                    self.witness(value)
                },
            }
        }
    }

    // Internal function that is given a subset of the sequence that it resends to itself after
    // resetting the index to 0.
    fn on_mismatch(&self, retry: &[T]) {
        self.index.set(0);
        for value in retry {
            self.witness(value);
            // I am a little concerned about a recursive loop since witness calls on_mismatch, but
            // I think it will be fine since the data sets that they retry are always decreasing.
            // I'll try to think of a sequence that will exercise this.
        }
    }
}

/// Creates a new [SequenceWatcher] from a [Vec].
///
/// # Examples
///
/// ```
/// # use seq_watcher::SequenceWatcher;
/// let seq = vec![b'X'];
/// let watcher = SequenceWatcher::from(seq);
///
/// assert_eq!(true, watcher.check(&b'X'));
/// ```
///
/// # Panics
///
/// Will panic if given an empty slice.
/// ```should_panic
/// # use seq_watcher::SequenceWatcher;
/// let seq: Vec<u32> = vec![];
/// let watcher = SequenceWatcher::from(seq);
/// ```
impl<T: PartialEq + Debug + Clone> From<Vec<T>> for SequenceWatcher<T> {
    fn from(src: Vec<T>) -> Self {
        assert!(!src.is_empty(), "Error: Can't create a SequenceWatcher with no sequence!");
        Self {
            sequence: src,
            index: Cell::new(0),
        }
    }
}

/// Creates a new [SequenceWatcher] from a slice.
///
/// # Examples
///
/// ```
/// # use seq_watcher::SequenceWatcher;
/// let watcher = SequenceWatcher::from(&[b'X'][..]);   // This is treated as a slice.
///
/// assert_eq!(true, watcher.check(&b'X'));
/// ```
///
/// This also works for array references for arrays of up to 32 itmes.
///
/// ```
/// # use seq_watcher::SequenceWatcher;
/// let watcher = SequenceWatcher::from(&[b'X']);   // This is treated as an array reference.
///
/// assert_eq!(true, watcher.check(&b'X'));
/// ```
///
/// # Panics
///
/// Will panic if given an empty slice.
/// ```should_panic
/// # use seq_watcher::SequenceWatcher;
/// let watcher: SequenceWatcher<u8> = SequenceWatcher::from(&[][..]);
/// ```
///
/// There is no support for a zero length array reference, so when using the array version, it will
/// fail to compile rather than panicing at runtime.
///
/// ```compile_fail
/// # use seq_watcher::SequenceWatcher;
/// let watcher: SequenceWatcher<u8> = SequenceWatcher::from(&[]);
/// ```
impl<T: PartialEq + Debug + Clone> From<&[T]> for SequenceWatcher<T> {
    fn from(src: &[T]) -> Self {
        SequenceWatcher::new(src)
    }
}

macro_rules! from_array {
    ($($s:expr),*) => {
        $(
            /// Creates a new [SequenceWatcher] from an array.
            impl<T: PartialEq + Debug + Clone> From<&[T; $s]> for SequenceWatcher<T> {
                fn from(src: &[T; $s]) -> Self {
                    Self::from(&src[..])
                }
            }
        )*
    }
}

from_array!(
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32
);

#[cfg(test)]
mod tests {
    use std::{
        fmt::Debug,
    };
    use proptest::{
        prelude::*,
        sample::SizeRange,
    };
    use crate::{SequenceWatcher, SequenceWatchers};

    #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
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
    fn values_and_stream<T: Eq + Clone + Debug>(sequence: Vec<T>, mut stream: Vec<T>) -> (Vec<T>, Vec<T>) {
        let seq_len = sequence.len();
        let mut i = 0;
        stream.extend_from_slice(&sequence);
        let mut not_good = true;
        while not_good {
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
            // On a rare occasion, this will accidentally create a duplicate sequence earlier than
            // it can otherwise detect, so we do a final samity check and check to see if we find
            // any early sequences, and if we do, we restart the filter.
            not_good = false;   // We start with the assumption that this will be okay.
            if stream.len() > seq_len {
                for x in 0..(stream.len()-1-seq_len) {
                    if &sequence[..] == &stream[x..(x+seq_len)] {
                        not_good = true;    // Oops. We found a match
                        i = 0;
                    }
                }
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
        let watcher = SequenceWatcher::new(sequence);
        let last_index = stream.len() - 1;
        for i in 0..last_index {
            prop_assert_eq!(false, watcher.check(&stream[i]));
        }
        prop_assert_eq!(true, watcher.check(&stream[last_index]));
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

    #[test]
    #[should_panic]
    fn empty_char_sequence_watcher_always_panics() {
        let watcher = SequenceWatcher::new(&[]);
        assert_eq!(false, watcher.check(&'a'));
    }

    #[test]
    #[should_panic]
    fn empty_byte_sequence_watcher_always_panics() {
        let watcher = SequenceWatcher::new(&[]);
        assert_eq!(false, watcher.check(&b'a'));
    }

    #[test]
    #[should_panic]
    fn empty_byte_or_char_sequence_watcher_always_panics() {
        let watcher = SequenceWatcher::new(&[]);
        assert_eq!(false, watcher.check(&'a'));
    }

    proptest! {
        #[test]
        fn empty_char_sequence_is_always_false(stream in char_vec(10..20)) {
            let watchers = SequenceWatchers::new(&[]);
            for c in stream {
                prop_assert_eq!(false, watchers.check(&c));
            }
        }
    }

    proptest! {
        #[test]
        fn empty_byte_sequence_is_always_false(stream in byte_vec(10..20)) {
            let watchers = SequenceWatchers::new(&[]);
            for b in stream {
                prop_assert_eq!(false, watchers.check(&b));
            }
        }
    }

    proptest! {
        #[test]
        fn empty_byte_or_char_sequence_is_always_false(stream in byte_or_char_vec(10..20)) {
            let watchers = SequenceWatchers::new(&[]);
            for c in stream {
                prop_assert_eq!(false, watchers.check(&c));
            }
        }
    }

    #[test]
    fn test_specific_patterns() {   // And also some different types.
        // This will fail if we just discard the state on a mismatch. On a failing case, when it
        // hits the second zero in the stream, it forgets about both zeroes and then doesn't match
        // the 1 because it is expecting a zero first.
        test_seq_watcher(&[false, true], &[false, false, true]).expect("Failed specific test pattern 1");
        // Let's say that we are smarter and pass the first test by retrying the current character
        // after resetting the index. what happens if we need a partial reset.
        test_seq_watcher(&[0u64, 0, 1], &[0, 0, 0, 1]).expect("Failed specific test pattern 2");
        // Now we need a pattern that will cause the simple fix for the second test to fail (or
        // maybe I don't need the more complicated solution).
        test_seq_watcher(&["F", "T", "F", "F", "F"],
            &["F", "T", "F", "F", "T", "F", "F", "F"]).expect("Failed specific test pattern 3");
        // Now we'll just repeat the last pattern with some alternate types.
        test_seq_watcher(&[None, Some(0.0), None, None, None],
            &[None, Some(0.0), None, None, Some(0.0), None, None, None])
            .expect("Failed specific test pattern 4");
        test_seq_watcher(&[Err(()), Ok(()), Err(()), Err(()), Err(())],
            &[Err(()), Ok(()), Err(()), Err(()), Ok(()), Err(()), Err(()), Err(())])
            .expect("Failed specific test pattern 5");
    }
}
