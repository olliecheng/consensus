// no lookup table will be produced - just hardcoded match statements
// this is probably faster for the compiler
// it should probably be rewritten to use a macro though...

use std::slice::{Chunks, ChunksExact};

pub trait Transformer<T> {
    fn transform(input: &[&T]) -> Self;
}

struct Pack<'a> {
    // s: &'a [&'a char],
    s: ChunksExact<'a, &'a char>
    done: bool,
}

impl<'a> Transformer<char> for Pack<'a> {
    fn transform(input: &[&char]) -> Self {
        Self {
            s: input.chunks_exact(4),
            done: false,
        }
    }
}

impl<'a> Iterator for Pack<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        if self.done {
            return None;
        }

        if let Some(v) = self.s.next() {
            v.into_iter().map(|x| {
                match x {
                    _ => 10
                }
            }).collect()            
        } else {
            self.s.remainder();
            todo!()
        }
        todo!();
        }
}

pub fn toChar<O>(input: &[&char]) -> O
where
    O: Iterator<Item = u8>,
{
    while Some(a) = input.chunks_exact(4) {}
    todo!();
}

pub fn toA<'a, I, O>(input: I) -> O
where
    I: Iterator<Item = &'a u8>,
    O: Iterator<Item = char>,
{
    todo!();
}
