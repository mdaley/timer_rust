use std::sync::mpsc;
use std::thread;
use std::sync::mpsc::{Sender, Receiver};

pub struct TriggeredWorker {
    tx_: Option<Sender<bool>>
}

impl TriggeredWorker {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            tx_: None
        }
    }

    #[allow(dead_code)]
    pub fn start(&mut self) {
        let channel: (Sender<bool>, Receiver<bool>) = mpsc::channel();
        let (tx, rx) = channel;
        self.tx_ = Some(tx);
        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(do_work) => {
                        match do_work {
                            true => {
                                println!("Work done!");
                            }
                            false => {
                                println!("Stopping");
                                break;
                            }
                        }
                    }
                    Err(_error) => {
                        // channel hung up -> stop
                        break;
                    }
                }
            }
        });
    }

    #[allow(dead_code)]
    pub fn stop(&mut self) {
        if let Some(s) = self.tx_.as_ref() {
            match s.send(false) {
                Ok(_result) => {
                    self.tx_ = None;
                }
                Err(_error) => {
                    // log an error and just get rid of the transmitter.
                    self.tx_ = None;
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn trigger(&mut self) {
        if let Some(s) = self.tx_.as_ref() {
            match s.send(true) {
                Ok(_result) => {}
                Err(_error) => {
                    // log an error
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
    use std::thread;

    #[test]
    fn construct() {
        let mut worker = TriggeredWorker::new();
        assert!(!worker.is_running());
        worker.start();
        assert!(worker.is_running());
        worker.stop();
        assert!(!worker.is_running());
    }

    #[test]
    fn do_some_work() {
        let mut work = TriggeredWorker::new();
        work.start();
        thread::sleep(Duration::from_secs(5));
        work.trigger();
        thread::sleep(Duration::from_secs(20));
    }

    #[test]
    fn do_lots_of_work() {
        let mut work = TriggeredWorker::new();
        work.start();

        for _i in 1..1000 {
            thread::sleep(Duration::from_millis(10));
            work.trigger();
        }

        thread::sleep(Duration::from_secs(5));
    }
}

