use crate::timer::TriggeredWorker;
use std::thread;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicU64};
use std::sync::atomic::Ordering::Relaxed;

mod timer;

fn main() {
    println!("How fast can worker run...");

    let i = Arc::new(AtomicI32::new(0));

    let time = Arc::new(AtomicU64::new(0));

    let mut worker = TriggeredWorker::new();

    let ii = i.clone();
    let thread_time = time.clone();
    worker.start(move || {
        let d = Instant::now();
        ii.fetch_add(1, Relaxed);
        thread_time.fetch_add(d.elapsed().as_nanos() as u64, Relaxed);
    });

    for _i in 0..100000 {
        worker.trigger();
    }

    thread::sleep(Duration::from_millis(10));

    println!("Work done count = {}", i.load(Relaxed));
    println!("Total thread time = {}ns", time.load(Relaxed));
    worker.stop();

    thread::sleep(Duration::from_millis(1));
}
