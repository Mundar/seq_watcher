Monitors a stream of data looking for a sequence (or multiple sequences)

# Stream Sequence Watcher

Includes a pair of structures, `SequenceWatcher` and `SequenceWatchers`. They monitor a stream
of data looking for a specific sequence of values.

The original purpose for this crate was to monitor bytes of data recieved from stdin for a
specific sequence that indicated that the input thread should shut down. Originally, only a
single sequence was monitored for, but I added a simple way to look for multiple sequences
simultaneously.

# Examples

## Intended Behavior and Limitations

* `SequenceWatcher` and `SequenceWatchers` both continue monitoring the stream after a check
  returns true. For example, looking for the sequence (A, A), would return false, true, true
  when given the input (A, A, A).

```rust
use seq_watcher::SequenceWatchers;

let test_stream = vec![
    ('A', false),
    ('A', false),
    ('A', true),    // Matches AAA
    ('A', true),    // Matches AAA
    ('B', false),
    ('A', true),    // Matches ABA
    ('B', false),
    ('A', true),    // Matches ABA
    ('A', false),
    ('A', true),    // Matches AAA
    ('C', true),    // Matches C
];

let watchers = SequenceWatchers::new(&[&['A', 'A', 'A'], &['A', 'B', 'A'], &['C']]);

for (ch, expect) in test_stream {
    assert_eq!(watchers.check(&ch), expect);
}

```

`SequenceWatcher` and `SequenceWatchers` that are given empty sequences always return false.

```rust
# use seq_watcher::SequenceWatchers;
let mut watcher = SequenceWatchers::new(&[]);  // Create empty watchers.
let mut watchers = SequenceWatchers::new(&[]);  // Create empty watchers.
let mut all_bytes = Vec::with_capacity(256);
for b in 0..=u8::MAX {
    assert_eq!(watcher.check(&b), false);   // With no sequence, all checks are false.
    assert_eq!(watchers.check(&b), false);  // With no sequences, all checks are false.
    all_bytes.push(b);                      // Generate sequence with all bytes.
}
watchers.add(&[]);          // Add of empty sequence does nothing.
watchers.add(&all_bytes);   // Add sequuence with all bytes.
for b in 0..u8::MAX {
    assert_eq!(watchers.check(&b), false);   // Until sequence recieves the last byte, false.
}
assert_eq!(watchers.check(&u8::MAX), true); // With last byte in sequence, returns true.
```

* Datatypes compatible are slightly more restrictive for `SequenceWatchers` than for
  `SequenceWatcher`. `SequenceWatchers` requires the datatype to be `Eq`, wheras
  `SequenceWatcher` only needs `PartialEq`.

Float types are `PartialEq` but not `Eq`.

```compile_fail
# use seq_watcher::SequenceWatchers;
let watchers = SequenceWatchers::new(&[0.0]);   // Float values are not Eq
```

```rust
# use seq_watcher::SequenceWatcher;
let watcher = SequenceWatcher::new(&[0.0]);     // Float values are PartialEq.
```

# Performance

The `SequenceWatcher` structure is resonably performant, but the `SequenceWatchers` structure
needs work. `SequenceWatchers` is currently implemented as a `HashMap` of `SequenceWatcher`
structures, but it would be better implemented with some sort of multi-state-aware trie.
`SequenceWatchers` was created as an afterthought, since I mainly needed the `SequenceWatcher`
for another project.
