use anyhow::{Context, Result};
use rayon::{iter::IterBridge, iter::ParallelBridge, prelude::ParallelIterator};
use std::fmt;
use std::hint::{self, spin_loop};
use std::marker::Sync;
use std::sync::mpsc;
use std::sync::{atomic, Arc};

#[derive(Debug, Clone)]
struct CacheTooSmallError {
    threads: usize,
    cache_size: usize,
}

impl fmt::Display for CacheTooSmallError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Cache size of {0} is too small for thread count of {1}. Ensure that cache size is at least equal to thread count", self.cache_size, self.threads)
    }
}

pub trait OrderedMapCollect<I, T>: Iterator<Item = I> + Send {
    fn par_map_and_emit(
        &mut self,
        map: impl (Fn(I) -> T) + Send + Sync,
        emit: impl FnMut(T) + Send,
        threads: usize,
        cache_size: usize,
    ) -> Result<()>;
}

impl<I: Iterator + Send, T: Send> OrderedMapCollect<I::Item, T> for I
where
    I::Item: Send,
{
    fn par_map_and_emit(
        &mut self,
        map: impl (Fn(I::Item) -> T) + Send + Sync,
        emit: impl FnMut(T) + Send,
        threads: usize,
        cache_size: usize,
    ) -> Result<()> {
        if cache_size < threads {
            return CacheTooSmallError {
                cache_size,
                threads,
            };
        }
        if threads == 1 {
            // single threaded execution is easy!
            self.map(map).for_each(emit);
        } else {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()?;

            pool.install(move || {
                // elements 0 to cache_size-1 (the first cache_size elements)
                // are all approved to emit at the start
                let approved_to_emit = Arc::new(atomic::AtomicUsize::new(cache_size - 1));
                let approved_to_emit_c = Arc::clone(&approved_to_emit);

                let (tx, rx) = mpsc::sync_channel::<(usize, T)>(cache_size);

                rayon::scope(|s| {
                    s.spawn(move |_| {
                        self.enumerate()
                            .par_bridge()
                            .map(|(i, value)| (i, map(value)))
                            .for_each_with(tx, |tx, (i, value)| {
                                debug!("Launch v {i}");
                                // wait until we are approved to emit
                                let approved_to_emit = Arc::clone(&approved_to_emit);

                                // is the index approved yet?
                                while approved_to_emit.load(atomic::Ordering::Acquire) < i {
                                    // debug!(
                                    //     "Approved: {}, i: {}",
                                    //     approved_to_emit.load(atomic::Ordering::Acquire),
                                    //     i
                                    // );
                                    hint::spin_loop();
                                }

                                debug!("Emit v {i}");
                                tx.send((i, value)).expect("Should succeed");
                            });
                    });

                    recover_order(rx, emit, approved_to_emit_c, cache_size);
                });
            });
        }
        Ok(())
    }
}

fn recover_order<T>(
    rx: mpsc::Receiver<(usize, T)>,
    mut emit: impl FnMut(T),
    approved_lock: Arc<atomic::AtomicUsize>,
    cache_size: usize,
) {
    let buffer: Vec<Option<T>> = (0..cache_size).map(|_| None).collect();
    // prevent resizing of the Vec
    let mut buffer = buffer.into_boxed_slice();

    let mut min_idx = 0_usize; // actual position of current index

    for (i, value) in rx {
        assert!(buffer[i % cache_size].is_none());
        buffer[i % cache_size] = Some(value);

        let mut emitted_item_count = 0 as usize;

        // check if we can emit anything in order
        for v in min_idx..(min_idx + cache_size) {
            // whenever we see a None, we can cancel and wait
            if buffer[v % cache_size].is_none() {
                break;
            }

            emitted_item_count += 1;
            min_idx += 1;
            let value = std::mem::replace(&mut buffer[v % cache_size], None).unwrap();
            emit(value);
        }

        approved_lock.store(min_idx, atomic::Ordering::Relaxed);
    }

    println!("Buffer capacity used: {}", cache_size);
}

fn set_threads(threads: u8) -> Result<()> {
    // set number of threads that Rayon uses
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads.into())
        .build_global()
        .with_context(|| format!("Unable to set the number of threads to {threads}"))
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as fmtWriter;
    use std::io::prelude::*;
    use std::io::BufWriter;

    use super::*;

    #[test]
    fn test_range() {
        let mut writer = BufWriter::new(Vec::new());

        let threads = 4;
        let result = (1..1000)
            .par_map_and_emit(
                |x| format!("Output {x}"),
                |x| writeln!(writer, "{}", x).unwrap(),
                threads,
                threads * 2,
            )
            .unwrap();

        // check result
        let bytes = writer.into_inner().unwrap();
        let str = String::from_utf8(bytes).unwrap();

        let mut expected_result = (1..1000)
            .map(|x| format!("Output {x}"))
            .collect::<Vec<_>>()
            .join("\n");
        expected_result.push_str("\n");

        assert_eq!(expected_result, str);
    }
}
