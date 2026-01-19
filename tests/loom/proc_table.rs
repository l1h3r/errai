use errai::core::InternalPid;
use errai::core::ProcTable;
use loom::thread;
use triomphe::Arc;

fn insert(table: &ProcTable<u64>, value: u64) -> InternalPid {
  let mut result: Option<InternalPid> = None;

  table
    .insert(|uninit, pid| {
      result = Some(pid);
      uninit.write(value);
    })
    .unwrap();

  result.unwrap()
}

#[test]
fn insert_during_remove_race() {
  loom::model(|| {
    let table: Arc<ProcTable<u64>> = Arc::new(ProcTable::new());
    let pid: InternalPid = insert(&table, 0);

    let t1 = {
      let table: Arc<ProcTable<u64>> = Arc::clone(&table);

      thread::spawn(move || {
        let del: Option<Arc<u64>> = table.remove(pid);
        assert!(del.is_some(), "Remove should succeed");
      })
    };

    let t2 = {
      let table: Arc<ProcTable<u64>> = Arc::clone(&table);

      thread::spawn(move || {
        let pid: InternalPid = insert(&table, 1);
        let get: Option<Arc<u64>> = table.get(pid);
        assert!(get.is_some(), "Newly inserted process not found!");
        pid
      })
    };

    t1.join().unwrap();
    let new_pid = t2.join().unwrap();

    assert!(table.get(new_pid).is_some(), "Process lost after race!");
    assert!(table.get(pid).is_none(), "Removed process still in table!");
  });
}

#[test]
fn concurrent_inserts_unique_pids() {
  loom::model(|| {
    let table: Arc<ProcTable<u64>> = Arc::new(ProcTable::new());

    let t1 = {
      let table: Arc<ProcTable<u64>> = Arc::clone(&table);

      thread::spawn(move || insert(&table, 0))
    };

    let t2 = {
      let table: Arc<ProcTable<u64>> = Arc::clone(&table);

      thread::spawn(move || insert(&table, 1))
    };

    let pid1: InternalPid = t1.join().unwrap();
    let pid2: InternalPid = t2.join().unwrap();

    assert_ne!(pid1, pid2, "Concurrent inserts produced same PID!");

    assert!(table.get(pid1).is_some());
    assert!(table.get(pid2).is_some());
  });
}

#[test]
fn concurrent_remove_same_pid() {
  loom::model(|| {
    let table: Arc<ProcTable<u64>> = Arc::new(ProcTable::new());
    let pid: InternalPid = insert(&table, 0);

    let t1 = {
      let table: Arc<ProcTable<u64>> = Arc::clone(&table);
      thread::spawn(move || table.remove(pid))
    };

    let t2 = {
      let table: Arc<ProcTable<u64>> = Arc::clone(&table);
      thread::spawn(move || table.remove(pid))
    };

    let r1 = t1.join().unwrap();
    let r2 = t2.join().unwrap();

    assert!(
      (r1.is_some() && r2.is_none()) || (r1.is_none() && r2.is_some()),
      "Both threads succeeded or both failed in removing same PID"
    );

    assert!(table.get(pid).is_none());
  });
}

#[test]
fn insert_lookup_race() {
  loom::model(|| {
    let table: Arc<ProcTable<u64>> = Arc::new(ProcTable::new());

    let t1 = {
      let table: Arc<ProcTable<u64>> = Arc::clone(&table);
      thread::spawn(move || insert(&table, 123))
    };

    let t2 = {
      let table: Arc<ProcTable<u64>> = Arc::clone(&table);
      let pid: InternalPid = InternalPid::from_bits(0);
      thread::spawn(move || table.get(pid))
    };

    let inserted_pid = t1.join().unwrap();
    let lookup_result = t2.join().unwrap();

    if let Some(found) = lookup_result {
      assert_eq!(*found, 123);
    }

    assert!(table.get(inserted_pid).is_some());
  });
}

#[ignore]
#[test]
fn rapid_insert_remove() {
  loom::model(|| {
    let table: Arc<ProcTable<u64>> = Arc::new(ProcTable::new());

    let t1 = {
      let table: Arc<ProcTable<u64>> = Arc::clone(&table);
      thread::spawn(move || table.remove(insert(&table, 0)))
    };

    let t2 = {
      let table: Arc<ProcTable<u64>> = Arc::clone(&table);
      thread::spawn(move || table.remove(insert(&table, 1)))
    };

    let p1 = t1.join().unwrap();
    let p2 = t2.join().unwrap();

    assert_eq!(table.is_empty(), true);
    assert_eq!(p1.as_deref(), Some(&0_u64));
    assert_eq!(p2.as_deref(), Some(&1_u64));
  });
}
