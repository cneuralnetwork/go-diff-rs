use crate::DiffMatchPatch;
use std::sync::Arc;

fn assert_send_and_sync<T: Send + Sync>() {}

#[test]
fn configuration_is_send_and_sync() {
    assert_send_and_sync::<DiffMatchPatch>();
}

#[test]
fn concurrent_diff_and_patch_soak() {
    let dmp = Arc::new(DiffMatchPatch::new());
    let workers: Vec<_> = (0..8)
        .map(|worker| {
            let dmp = Arc::clone(&dmp);
            std::thread::spawn(move || {
                for case in 0..200 {
                    let first = format!("worker={worker}\ncase={case}\nalpha beta gamma\n");
                    let second = format!("worker={worker}\ncase={}\nalpha delta gamma\n", case + 1);
                    let diffs = dmp.diff_main(&first, &second, case % 2 == 0);
                    assert_eq!(dmp.diff_text1(&diffs), first.as_str());
                    assert_eq!(dmp.diff_text2(&diffs), second.as_str());
                    let patches = dmp.patch_make(&first, &second);
                    let (actual, applied) = dmp.patch_apply(&patches, &first);
                    assert_eq!(actual, second.as_str());
                    assert!(applied.iter().all(|value| *value));
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("concurrent worker");
    }
}
