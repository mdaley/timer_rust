extern crate custom_error;

use custom_error::custom_error;
use std::sync::mpsc;
use std::thread;
use std::sync::mpsc::{Receiver, SendError, Sender};
use crate::timer::TimerError::{AlreadyStopped, NotRunning, AlreadyStarted};

custom_error! {#[derive(PartialEq)] pub TimerError
    AlreadyStopped = "Cannot stop an already stopped worker",
    AlreadyStarted = "Cannot start and already started worker",
    NotRunning = "Cannot trigger on a worker that is not running",
    SendFailure{source: SendError<bool>} = "Cannot send message"
}

pub struct TriggeredWorker {
    tx_: Option<Sender<bool>>
}

impl TriggeredWorker {
    pub fn new() -> Self {
        Self {
            tx_: None
        }
    }

    pub fn start<F: 'static + Fn() + Send>(&mut self, work: F) -> Result<(), TimerError> {
        if self.tx_.is_some() {
            Err(AlreadyStarted)?
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
                    Err(_error) => {
                        // maybe do logging? Can't report back as this thread just ends. Not using
                        // the JoinHandle... Right or wrong?
                    }
                }
            }
        });

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), TimerError> {
        if self.tx_.is_none() {
            Err(AlreadyStopped)?
        }

        if let Some(s) = self.tx_.as_ref() {
            match s.send(false) {
                Ok(_result) => {
                    self.tx_ = None;
                }
                Err(error) => {
                    Err(error)?
                }
            }
        }

        Ok(())
    }

    pub fn trigger(&mut self) -> Result<(), TimerError> {
        if self.tx_.is_none() {
            Err(NotRunning)?
        }

        if let Some(s) = self.tx_.as_ref() {
            match s.send(true) {
                Ok(_result) => {}
                Err(error) => {
                    Err(error)?
                }
            }
        }

        Ok(())
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
    use std::sync::{Arc, mpsc};
    use crate::timer::TimerError::{AlreadyStopped, SendFailure, NotRunning, AlreadyStarted};
    use std::sync::mpsc::{SendError};

    #[test]
    fn work_can_only_be_started_once() {
        let mut worker = TriggeredWorker::new();
        let _ = worker.start(|| {});
        assert_eq!(worker.start(|| {}).unwrap_err(), AlreadyStarted);
    }

    #[test]
    fn work_can_only_be_stopped_once() {
        let mut worker = TriggeredWorker::new();
        let _ = worker.start(|| {});
        assert!(worker.stop().is_ok());
        assert_eq!(worker.stop().unwrap_err(), AlreadyStopped);
    }

    #[test]
    fn stopping_when_channel_already_stopped_results_in_error() {
        let mut worker = TriggeredWorker::new();
        let _ = worker.start(|| {});

        // give the worker a different sender that isn't linked via the channel to the receiver
        // in the worker's thread. This makes the channel close because the previous sender is
        // dropped. However, there is a sender available so that stop() doesn't immediately fail
        // with AlreadyStopped.
        worker.tx_ = Some(mpsc::channel().0);
        assert_eq!(worker.stop(), Err(SendFailure { source: SendError(false) }));

    }

    #[test]
    fn can_not_trigger_on_a_worker_that_is_not_running() {
        let mut worker = TriggeredWorker::new();
        assert_eq!(worker.trigger().unwrap_err(), NotRunning);
    }

    #[test]
    fn trigger_fails_if_channel_has_broken() {
        let mut worker = TriggeredWorker::new();
        let _ = worker.start(|| {});

        worker.tx_ = Some(mpsc::channel().0);
        assert_eq!(worker.trigger(), Err(SendFailure { source: SendError(true) }));
    }

    #[test]
    fn can_start_stop_and_then_start_worker() {
        let done_some_work = Arc::new(AtomicBool::new(false));
        let internal_done_some_work = done_some_work.clone();
        let mut worker = TriggeredWorker::new();
        let _ = worker.start(|| {});
        let _ = worker.stop();
        let _ = worker.start(move || {
            internal_done_some_work.store(true, Relaxed);
        });
        let _ = worker.trigger();
        thread::sleep(Duration::from_millis(10));

        let _ = worker.stop();
        assert!(done_some_work.load(Relaxed));
    }

    #[test]
    fn one_piece_of_work_can_be_triggered() {
        let done_some_work = Arc::new(AtomicBool::new(false));
        let mut worker = TriggeredWorker::new();

        assert!(!worker.is_running());

        let internal_done_some_work = done_some_work.clone();
        let _ = worker.start(move || {
            internal_done_some_work.store(true, Relaxed);
        });

        assert!(worker.is_running());

        let _ = worker.trigger();

        // make sure worker has time to run!
        thread::sleep(Duration::from_millis(10));

        let _ = worker.stop();

        assert!(!worker.is_running());
        assert!(done_some_work.load(Relaxed));
    }

    #[test]
    fn nothing_happens_if_work_is_not_triggered() {
        let done_some_work = Arc::new(AtomicBool::new(false));

        let mut worker = TriggeredWorker::new();

        let internal_done_some_work = done_some_work.clone();
        let _ = worker.start(move|| {
            internal_done_some_work.store(true, Relaxed);
        });

        thread::sleep(Duration::from_millis(10));

        let _ = worker.stop();

        assert!(!worker.is_running());
        assert!(!done_some_work.load(Relaxed));

    }

    #[test]
    fn do_lots_of_work() {
        let work_count = Arc::new(AtomicI32::new(0));
        let mut worker = TriggeredWorker::new();
        let internal_work_count = work_count.clone();

        let _ = worker.start(move|| {
            internal_work_count.fetch_add(1, Relaxed);
        });

        for _i in 0..1000 {
            let _ = worker.trigger();
        }

        thread::sleep(Duration::from_millis(10));

        let _ = worker.stop();

        assert_eq!(work_count.load(Relaxed), 1000);
    }
}

