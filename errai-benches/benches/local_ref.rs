use criterion::BenchmarkGroup;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use errai::core::LocalRef;
use std::hint::black_box;
use std::sync::Arc;
use std::sync::Barrier;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

const THREADS: &[usize] = &[2, 4, 6, 8, 10];

fn bench_local_ref(criterion: &mut Criterion) {
  let mut group: BenchmarkGroup<_> = criterion.benchmark_group("local_ref");

  group.bench_function("single-threaded", |bench| {
    bench.iter(|| {
      black_box(LocalRef::new());
    })
  });

  for threads in THREADS {
    let id: BenchmarkId = BenchmarkId::new("multi-threaded", threads);

    group.bench_with_input(id, threads, |bench, &threads| {
      bench.iter_custom(|iters| {
        let barrier: Arc<Barrier> = Arc::new(Barrier::new(threads + 1));
        let mut handles: Vec<JoinHandle<Duration>> = Vec::with_capacity(threads);

        for _ in 0..threads {
          let barrier: Arc<Barrier> = barrier.clone();

          let handle: JoinHandle<Duration> = thread::spawn(move || {
            barrier.wait();

            let start: Instant = Instant::now();

            for _ in 0..iters {
              black_box(LocalRef::new());
            }

            start.elapsed()
          });

          handles.push(handle);
        }

        barrier.wait();

        handles
          .into_iter()
          .map(|handle| handle.join().unwrap())
          .sum()
      })
    });
  }

  group.finish();
}

criterion_group! {
  name = benches;
  config = Criterion::default();
  targets = bench_local_ref
}

criterion_main!(benches);
