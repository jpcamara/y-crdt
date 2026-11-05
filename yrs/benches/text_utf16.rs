use criterion::*;
use std::time::{Duration, Instant};
use yrs::{Doc, OffsetKind, Options, Text, Transact};

// Single-character inserts at the end of a text inserted in one go (so it's held as a single
// item), each in its own transaction, with UTF-16 offsets. See #668.
fn bench_insert_at_end(c: &mut Criterion) {
    let mut g = c.benchmark_group("text_utf16/insert_at_end");
    g.sample_size(20);
    for chars in [10_000usize, 100_000, 1_000_000, 4_000_000] {
        let initial: String = "lorem ipsum dolor sit amet é "
            .chars()
            .cycle()
            .take(chars)
            .collect();
        // rebuild the doc (untimed) before the inserts grow the text by more than 1%
        let max_inserts = (chars / 100) as u64;
        g.bench_with_input(BenchmarkId::from_parameter(chars), &initial, |b, initial| {
            b.iter_custom(|iters| {
                let mut elapsed = Duration::ZERO;
                let mut remaining = iters;
                while remaining > 0 {
                    let doc = Doc::with_options(Options {
                        offset_kind: OffsetKind::Utf16,
                        ..Options::default()
                    });
                    let text = doc.get_or_insert_text("text");
                    text.insert(&mut doc.transact_mut(), 0, initial);
                    let batch = remaining.min(max_inserts);
                    let start = Instant::now();
                    for _ in 0..batch {
                        let mut txn = doc.transact_mut();
                        let end = text.len(&txn);
                        text.insert(&mut txn, end, "x");
                    }
                    elapsed += start.elapsed();
                    remaining -= batch;
                }
                elapsed
            });
        });
    }
    g.finish();
}

// Single-character appends to a text, each in its own transaction, with an update observer (as
// sync providers use). The appends are squashed into one item, and each update holds only the
// newest character.
fn bench_append_with_update_observer(c: &mut Criterion) {
    let mut g = c.benchmark_group("text_utf16/append_with_update_observer");
    g.sample_size(10);
    for appends in [10_000usize, 50_000] {
        for (name, ch) in [("ascii", "x"), ("non_ascii", "é")] {
            g.bench_function(BenchmarkId::new(name, appends), |b| {
                b.iter(|| {
                    let doc = Doc::with_options(Options {
                        offset_kind: OffsetKind::Utf16,
                        ..Options::default()
                    });
                    doc.observe_update_v1("bench", |_, e| {
                        black_box(e.update.len());
                    })
                    .unwrap();
                    let text = doc.get_or_insert_text("text");
                    for _ in 0..appends {
                        let mut txn = doc.transact_mut();
                        let end = text.len(&txn);
                        text.insert(&mut txn, end, ch);
                    }
                });
            });
        }
    }
    g.finish();
}

criterion_group! {
    name = text_utf16_benches;
    config = Criterion::default();
    targets = bench_insert_at_end, bench_append_with_update_observer,
}
criterion_main!(text_utf16_benches);
