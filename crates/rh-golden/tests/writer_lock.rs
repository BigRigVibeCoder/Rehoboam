//! The run directory's writer lock (VER-SPR-013-TEST GAP-11).
//!
//! - A reader's brief shared lock (`tail --wait`'s probe) does not refuse a writer that starts in
//!   the same instant: the writer retries with a bounded backoff and takes the lock.
//! - A held exclusive lock (a second writer) is still refused after the bound, naming the pid the
//!   holder recorded and whether it is running.
//! - The `flock` decides, not the file's content: a lock file naming a dead pid, with no lock held,
//!   is taken, and the pid is overwritten with the taker's.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers fail loudly"
)]

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use rh_golden::stream::{WRITER_LOCK, lock_writer};

/// A pid that cannot be running: Linux pids are below `pid_max`, at most 2^22.
const DEAD_PID: u32 = 4_194_304;

fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("writer_lock")
        .join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_brief_shared_lock_does_not_refuse_a_starting_writer() {
    let dir = scratch("shared");
    std::fs::write(dir.join(WRITER_LOCK), b"").unwrap();
    let (held, wait) = mpsc::channel();
    let probe = {
        let path = dir.join(WRITER_LOCK);
        std::thread::spawn(move || {
            let f = std::fs::File::open(path).unwrap();
            f.try_lock_shared().unwrap();
            held.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(20));
        })
    };
    wait.recv().unwrap();
    let lock = lock_writer(&dir);
    probe.join().unwrap();
    assert!(lock.is_ok(), "{:?}", lock.err());
    println!("GAP-11: a writer starting under a 20 ms shared probe takes the lock");
}

#[test]
fn a_second_writer_is_refused_naming_the_holder() {
    let dir = scratch("exclusive");
    let _first = lock_writer(&dir).unwrap();
    let err = lock_writer(&dir).unwrap_err();
    let mine = format!("held by pid {} (running)", std::process::id());
    assert!(
        err.0.contains("another run is writing this run directory") && err.0.contains(&mine),
        "{err}"
    );
    let dead = scratch("dead_holder");
    let path = dead.join(WRITER_LOCK);
    std::fs::write(&path, format!("pid {DEAD_PID}\nstarted 0\n")).unwrap();
    let held = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    held.try_lock().unwrap();
    let err = lock_writer(&dead).unwrap_err();
    assert!(
        err.0
            .contains(&format!("held by pid {DEAD_PID} (not running)")),
        "{err}"
    );
    println!("GAP-11: a held lock is refused after the bound, naming the holder's pid and state");
}

#[test]
fn a_lock_file_naming_a_dead_pid_is_taken_when_no_lock_is_held() {
    let dir = scratch("stale");
    let path = dir.join(WRITER_LOCK);
    std::fs::write(&path, format!("pid {DEAD_PID}\nstarted 0\n")).unwrap();
    let _lock = lock_writer(&dir).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.starts_with(&format!("pid {}\n", std::process::id())),
        "{text}"
    );
    println!("GAP-11: the flock decides; a stale pid in the file is overwritten by the taker's");
}
