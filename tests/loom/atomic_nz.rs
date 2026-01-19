use errai::tyre::num::AtomicNzU64;
use loom::sync::Arc;
use loom::sync::atomic::Ordering;
use loom::thread;

#[test]
fn new_rejects_zero() {
  loom::model(|| {
    let result: Option<AtomicNzU64> = AtomicNzU64::new(0);
    assert!(result.is_none(), "new(0) should return None");

    let result: Option<AtomicNzU64> = AtomicNzU64::new(1);
    assert!(result.is_some(), "new(1) should return Some");
  });
}

#[test]
fn fetch_add_never_zero() {
  loom::model(|| {
    let atomic: Arc<AtomicNzU64> = Arc::new(AtomicNzU64::new(u64::MAX - 5).unwrap());

    let threads: Vec<_> = (0..2)
      .map(|_| {
        let atomic: Arc<AtomicNzU64> = Arc::clone(&atomic);

        thread::spawn(move || {
          for _ in 0..3 {
            let old: u64 = atomic.fetch_add(3, Ordering::Relaxed).get();
            assert_ne!(old, 0, "fetch_add returned zero!");

            let new: u64 = atomic.load(Ordering::Relaxed).get();
            assert_ne!(new, 0, "load() returned zero!");
          }
        })
      })
      .collect();

    for handle in threads {
      handle.join().unwrap();
    }

    let out: u64 = atomic.load(Ordering::Relaxed).get();
    assert_ne!(out, 0, "Final value is zero!");
  });
}

#[test]
fn concurrent_fetch_add_and_load() {
  loom::model(|| {
    let atomic: Arc<AtomicNzU64> = Arc::new(AtomicNzU64::new(1).unwrap());

    let t1 = {
      let atomic: Arc<AtomicNzU64> = Arc::clone(&atomic);

      thread::spawn(move || {
        atomic.fetch_add(5, Ordering::Relaxed);
      })
    };

    let t2 = {
      let atomic: Arc<AtomicNzU64> = Arc::clone(&atomic);

      thread::spawn(move || {
        for _ in 0..2 {
          let val: u64 = atomic.load(Ordering::Relaxed).get();
          assert_ne!(val, 0, "Observed zero during concurrent load!");
        }
      })
    };

    t1.join().unwrap();
    t2.join().unwrap();
  });
}

#[test]
fn wrapping_with_multiple_threads() {
  loom::model(|| {
    let atomic: Arc<AtomicNzU64> = Arc::new(AtomicNzU64::new(u64::MAX - 2).unwrap());

    let threads: Vec<_> = (0..2)
      .map(|_| {
        let atomic: Arc<AtomicNzU64> = Arc::clone(&atomic);

        thread::spawn(move || {
          let old: u64 = atomic.fetch_add(u64::MAX / 2, Ordering::Relaxed).get();
          assert_ne!(old, 0);

          let new: u64 = atomic.load(Ordering::Relaxed).get();
          assert_ne!(new, 0, "Value wrapped to zero!");
        })
      })
      .collect();

    for handle in threads {
      handle.join().unwrap();
    }
  });
}

#[test]
fn high_contention() {
  loom::model(|| {
    let atomic = Arc::new(AtomicNzU64::new(100).unwrap());

    let threads: Vec<_> = (0..3)
      .map(|_| {
        let atomic: Arc<AtomicNzU64> = Arc::clone(&atomic);

        thread::spawn(move || {
          for _ in 0..2 {
            let old: u64 = atomic.fetch_add(1, Ordering::Relaxed).get();
            assert_ne!(old, 0);

            let new: u64 = atomic.load(Ordering::Relaxed).get();
            assert_ne!(new, 0);
          }
        })
      })
      .collect();

    for handle in threads {
      handle.join().unwrap();
    }
  });
}
