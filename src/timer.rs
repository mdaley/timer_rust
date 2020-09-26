use std::sync::mpsc;
use std::thread;
use std::sync::mpsc::{Sender, Receiver};

pub struct TriggeredWorker {
    tx_: Option<Sender<bool>>
}

impl TriggeredWorker {
    pub fn new() -> Self {
        Self {
            tx_: None
        }
    }

    pub fn start<F: 'static + Fn() + Send>(&mut self, work: F) {
        if self.tx_.is_some() {
            println!("*** can't start");
            return;
        }

        let channel: (Sender<bool>, Receiver<bool>) = mpsc::channel();

        let (tx, rx) = channel;
        self.tx_ = Some(tx);
        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(do_work) => {
                        match do_work {
                            true => {
                                work();
                            }
                            false => {
                                break;
                            }
                        }
                    }
                    Err(error) => {
                        println!("*** {}", error.to_string());
                    }
                }
            }
        });
    }

    pub fn stop(&mut self) {
        if self.tx_.is_none() {
            panic!("Can't stop an already stopped worker! You must start it first.");
        }

        if let Some(s) = self.tx_.as_ref() {
            match s.send(false) {
                Ok(_result) => {
                    self.tx_ = None;
                }
                Err(error) => {
                    panic!(error);
                }
            }
        }
    }

    pub fn trigger(&mut self) {
        if self.tx_.is_none() {
            panic!("Can't trigger on a worker that is not running.");
        }

        if let Some(s) = self.tx_.as_ref() {
            match s.send(true) {
                Ok(_result) => {}
                Err(error) => {
                    panic!(error);
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.tx_.is_some()
    }
}

#[cfg(test)]
mod tests {
    use crate::timer::TriggeredWorker;
    use std::time::Duration;
    use std::{thread, panic};
    use std::sync::atomic::{AtomicI32, AtomicBool};
    use std::sync::atomic::Ordering::Relaxed;
    use std::sync::Arc;

    // Tidy up test output by hiding panic traces and just showing to_string output.
    fn hide_panic<F: Fn()>(ftn: F) {
        let std_panic_hook = panic::take_hook();
        panic::set_hook(Box::new(|a| {
            println!("{}", a.to_string());
        }));
        ftn();
        panic::set_hook(std_panic_hook);
    }

    #[test]
    //#[should_panic (expected = "already started")]
    fn work_can_only_be_started_once() {
        hide_panic(|| {
            let mut worker = TriggeredWorker::new();
            worker.start(|| { println!("worker 1"); });
            //thread::sleep(Duration::from_millis(100));
            worker.start(|| { println!("worker 2"); });

            worker.stop();
        });
    }

    #[test]
    #[should_panic (expected = "already stopped")]
    fn work_can_only_be_stopped_once() {
        //hide_panic(|| {
            let mut worker = TriggeredWorker::new();
            worker.start(|| {});
            worker.stop();
            worker.stop();
        //});
    }

    #[test]
    #[should_panic (expected = "not running")]
    fn can_not_trigger_on_a_worker_that_is_not_running() {
        hide_panic(|| {
            let mut worker = TriggeredWorker::new();
            worker.trigger();
        });
    }

    #[test]
    fn can_start_stop_and_then_start_worker() {
        let done_some_work = Arc::new(AtomicBool::new(false));
        let internal_done_some_work = done_some_work.clone();
        let mut worker = TriggeredWorker::new();
        worker.start(|| {});
        worker.stop();
        worker.start(move || {
            internal_done_some_work.store(true, Relaxed);
        });
        worker.trigger();
        thread::sleep(Duration::from_millis(10));

        worker.stop();
        assert!(done_some_work.load(Relaxed));
    }

    #[test]
    fn one_piece_of_work_can_be_triggered() {
        let done_some_work = Arc::new(AtomicBool::new(false));
        let mut worker = TriggeredWorker::new();

        assert!(!worker.is_running());

        let internal_done_some_work = done_some_work.clone();
        worker.start(move || {
            internal_done_some_work.store(true, Relaxed);
        });

        assert!(worker.is_running());

        worker.trigger();

        // make sure worker has time to run!
        thread::sleep(Duration::from_millis(10));

        worker.stop();

        assert!(!worker.is_running());
        assert!(done_some_work.load(Relaxed));
    }

    #[test]
    fn nothing_happens_if_work_is_not_triggered() {
        let done_some_work = Arc::new(AtomicBool::new(false));

        let mut worker = TriggeredWorker::new();

        let internal_done_some_work = done_some_work.clone();
        worker.start(move|| {
            internal_done_some_work.store(true, Relaxed);
        });

        thread::sleep(Duration::from_millis(10));

        worker.stop();

        assert!(!worker.is_running());
        assert!(!done_some_work.load(Relaxed));

    }

    #[test]
    fn do_lots_of_work() {
        let work_count = Arc::new(AtomicI32::new(0));
        let mut worker = TriggeredWorker::new();
        let internal_work_count = work_count.clone();

        worker.start(move|| {
            internal_work_count.fetch_add(1, Relaxed);
        });

        for _i in 0..1000 {
            worker.trigger();
        }

        thread::sleep(Duration::from_millis(10));

        worker.stop();

        assert_eq!(work_count.load(Relaxed), 1000);
    }
}

