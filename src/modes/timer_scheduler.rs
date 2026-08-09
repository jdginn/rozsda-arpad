use crossbeam_channel::{Receiver, Sender, select, tick};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimerKey {
    pub encoder_idx: u8, // 0..15
}

#[derive(Debug, Clone)]
pub enum SchedulerCmd<T> {
    Schedule {
        key: TimerKey,
        delay: Duration,
        msg: T,
    },
    Cancel {
        key: TimerKey,
    },
    ClearAll,
    Shutdown,
}

#[derive(Debug, Clone)]
struct Pending<T> {
    due: Instant,
    msg: T,
}

pub fn run_scheduler<T: Send + Clone + 'static>(
    cmd_rx: Receiver<SchedulerCmd<T>>,
    out_tx: Sender<T>,
    poll_every: Duration, // e.g. 25-50ms
) {
    let ticker = tick(poll_every);
    let mut pending: HashMap<TimerKey, Pending<T>> = HashMap::new();

    loop {
        select! {
            recv(cmd_rx) -> cmd => match cmd {
                Ok(SchedulerCmd::Schedule { key, delay, msg }) => {
                    // replace existing timer for this key (debounce/latest-wins)
                    pending.insert(key, Pending { due: Instant::now() + delay, msg });
                }
                Ok(SchedulerCmd::Cancel { key }) => {
                    pending.remove(&key);
                }
                // Ok(SchedulerCmd::CancelMode { mode_id }) => {
                //     pending.retain(|k, _| k.mode_id != mode_id);
                // }
                Ok(SchedulerCmd::ClearAll) => {
                    pending.clear();
                }
                Ok(SchedulerCmd::Shutdown) | Err(_) => break,
            },
            recv(ticker) -> _ => {
                let now = Instant::now();

                // collect due keys first to avoid borrow issues
                let due_keys: Vec<TimerKey> = pending.iter()
                    .filter_map(|(k, p)| (p.due <= now).then_some(k.clone()))
                    .collect();

                for key in due_keys {
                    if let Some(p) = pending.remove(&key) {
                        // if receiver is gone, shut down scheduler
                        if out_tx.send(p.msg).is_err() {
                            return;
                        }
                    }
                }
            }
        }
    }
}
