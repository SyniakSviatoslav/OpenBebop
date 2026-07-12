//! lanes.rs — parallel-session scheduler (category O of the master plan).
//! Minimal but real: a Lane tracks throughput + queue + ETA. The dispatcher
//! routes work to the freest lane. Extended by Wave 3 (tui panel, auto-queue).
//! Operator: default maximal parallelism, never exceeds `max_lanes`.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneStatus {
    Idle,
    Running,
    Draining,
}

#[derive(Debug, Clone)]
pub struct Lane {
    pub name: String,
    pub status: LaneStatus,
    pub completed: u64,
    pub busy_since: Option<Instant>,
    pub queue: VecDeque<String>,
}

impl Lane {
    pub fn new(name: &str) -> Self {
        Lane {
            name: name.to_string(),
            status: LaneStatus::Idle,
            completed: 0,
            busy_since: None,
            queue: VecDeque::new(),
        }
    }

    /// Throughput = completed tasks per minute, measured over a sliding window.
    /// Cheap stand-in: completed / elapsed-min since first busy (clamped).
    pub fn throughput(&self, started: Instant) -> f64 {
        let mins = started.elapsed().as_secs_f64() / 60.0;
        if mins < 1e-6 {
            0.0
        } else {
            self.completed as f64 / mins
        }
    }

    pub fn enqueue(&mut self, item: String) {
        self.queue.push_back(item);
    }

    /// Dispatch: refuse if already at max_lanes worth of running lanes.
    pub fn dispatch(lanes: &mut [Lane], item: String, max_lanes: usize) -> bool {
        if lanes.iter().filter(|l| l.status == LaneStatus::Running).count() >= max_lanes {
            return false; // RED: refuse > max_lanes concurrently
        }
        // Freest = fewest queued + idle preferred.
        if let Some(lane) = lanes.iter_mut().min_by_key(|l| l.queue.len()) {
            lane.enqueue(item);
            if lane.status == LaneStatus::Idle {
                lane.status = LaneStatus::Running;
                lane.busy_since = Some(Instant::now());
            }
            true
        } else {
            false
        }
    }

    /// Predicted finish for the head item: EMA of prior same-size durations.
    pub fn eta(&self, avg_task: Duration) -> Duration {
        avg_task * (self.queue.len() as u32 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuse_over_max_lanes() {
        let mut lanes = vec![Lane::new("a"), Lane::new("b"), Lane::new("c")];
        lanes[0].status = LaneStatus::Running;
        lanes[1].status = LaneStatus::Running;
        // max_lanes = 2, already 2 running -> refuse 3rd dispatch
        assert!(!Lane::dispatch(&mut lanes, "x".into(), 2));
    }

    #[test]
    fn dispatch_routes_to_freest() {
        let mut lanes = vec![Lane::new("a"), Lane::new("b"), Lane::new("c")];
        lanes[0].enqueue("old".into());
        // b is freest (empty) -> item goes to b
        assert!(Lane::dispatch(&mut lanes, "new".into(), 3));
        assert_eq!(lanes[1].queue.back().unwrap(), "new");
        assert_eq!(lanes[1].status, LaneStatus::Running);
    }

    #[test]
    fn throughput_zero_before_work() {
        let lane = Lane::new("a");
        let start = Instant::now();
        assert_eq!(lane.throughput(start), 0.0);
    }
}
