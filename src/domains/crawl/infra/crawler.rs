use std::collections::HashMap;
use std::sync::mpsc as std_mpsc;

use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::domains::crawl::dto::task_dto::SseEvent;

/// Commands sent from `CrawlTaskManager` to the dedicated crawl runner thread.
#[derive(Debug)]
pub enum RunnerCommand {
    Execute { task_id: i64 },
    Shutdown,
}

/// Manages the crawl task queue and runner.
/// Wrapped in Arc<Mutex<>> for shared access.
pub struct CrawlTaskManager {
    current_task: Option<i64>,
    queue: Vec<i64>,
    cancellation_tokens: HashMap<i64, CancellationToken>,
    broadcast_tx: broadcast::Sender<SseEvent>,
    runner_tx: Option<std_mpsc::Sender<RunnerCommand>>,
}

impl CrawlTaskManager {
    pub fn new(broadcast_tx: broadcast::Sender<SseEvent>) -> Self {
        Self {
            current_task: None,
            queue: Vec::new(),
            cancellation_tokens: HashMap::new(),
            broadcast_tx,
            runner_tx: None,
        }
    }

    pub fn set_runner_tx(&mut self, tx: std_mpsc::Sender<RunnerCommand>) {
        self.runner_tx = Some(tx);
    }

    pub fn broadcast_tx(&self) -> &broadcast::Sender<SseEvent> {
        &self.broadcast_tx
    }

    pub fn current_task(&self) -> Option<i64> {
        self.current_task
    }

    pub fn is_idle(&self) -> bool {
        self.current_task.is_none() && self.queue.is_empty()
    }

    /// Returns true if the task was started immediately (no current task).
    /// When true, sends a `RunnerCommand::Execute` to the runner thread.
    pub fn enqueue(&mut self, task_id: i64) -> bool {
        if self.current_task.is_none() {
            self.current_task = Some(task_id);
            if let Some(tx) = &self.runner_tx {
                drop(tx.send(RunnerCommand::Execute { task_id }));
            }
            true
        } else {
            self.queue.push_back(task_id);
            false
        }
    }

    /// Completes the current task and optionally dispatches the next queued task.
    /// Returns the next task ID if one was dispatched.
    pub fn complete_current(&mut self) -> Option<i64> {
        if let Some(old_id) = self.current_task.take() {
            self.cancellation_tokens.remove(&old_id);
        }
        if let Some(next_id) = self.queue.pop_front() {
            self.current_task = Some(next_id);
            if let Some(tx) = &self.runner_tx {
                drop(tx.send(RunnerCommand::Execute { task_id: next_id }));
            }
            Some(next_id)
        } else {
            None
        }
    }

    pub fn cancel_task(&mut self, task_id: i64) -> CancelAction {
        if self.current_task == Some(task_id) {
            if let Some(token) = self.cancellation_tokens.get(&task_id) {
                token.cancel();
            } else {
                // Token not installed yet — insert a pre-cancelled one.
                // set_cancellation_token will propagate the cancel state
                // when it installs the real token later.
                let token = CancellationToken::new();
                token.cancel();
                self.cancellation_tokens.insert(task_id, token);
            }
            CancelAction::Running
        } else if let Some(pos) = self.queue.iter().position(|&id| id == task_id) {
            self.queue.remove(pos);
            self.cancellation_tokens.remove(&task_id);
            CancelAction::RemovedFromQueue
        } else {
            CancelAction::NotFound
        }
    }

    pub fn set_cancellation_token(&mut self, task_id: i64, token: CancellationToken) {
        // If the task was cancelled before the token was installed, propagate.
        if self
            .cancellation_tokens
            .get(&task_id)
            .is_some_and(|t| t.is_cancelled())
        {
            token.cancel();
        }
        self.cancellation_tokens.insert(task_id, token);
    }

    pub fn is_cancelled(&self, task_id: i64) -> bool {
        self.cancellation_tokens
            .get(&task_id)
            .is_some_and(|t| t.is_cancelled())
    }

    pub fn emit_event(&self, event: SseEvent) {
        // Ignore send errors (no receivers)
        drop(self.broadcast_tx.send(event));
    }

    /// Startup reconciliation: re-enqueue queued task IDs and dispatch the first.
    /// Running tasks should be marked failed externally before calling this.
    pub fn reconcile_startup(&mut self, queued_task_ids: Vec<i64>) {
        for id in queued_task_ids {
            if self.current_task.is_none() {
                self.current_task = Some(id);
                if let Some(tx) = &self.runner_tx {
                    drop(tx.send(RunnerCommand::Execute { task_id: id }));
                }
            } else {
                self.queue.push_back(id);
            }
        }
    }
}

pub enum CancelAction {
    Running,
    RemovedFromQueue,
    NotFound,
}

// VecDeque helper methods
trait VecDequeExt<T> {
    fn push_back(&mut self, val: T);
    fn pop_front(&mut self) -> Option<T>;
}

impl<T> VecDequeExt<T> for Vec<T> {
    fn push_back(&mut self, val: T) {
        self.push(val);
    }
    fn pop_front(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            Some(self.remove(0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn new_manager() -> CrawlTaskManager {
        let (tx, _rx) = broadcast::channel(16);
        CrawlTaskManager::new(tx)
    }

    #[test]
    fn enqueue_first_task_starts_immediately() {
        let mut mgr = new_manager();
        assert!(mgr.enqueue(1));
        assert_eq!(mgr.current_task(), Some(1));
        assert!(!mgr.is_idle());
    }

    #[test]
    fn enqueue_second_task_queues() {
        let mut mgr = new_manager();
        assert!(mgr.enqueue(1));
        assert!(!mgr.enqueue(2));
        assert_eq!(mgr.current_task(), Some(1));
    }

    #[test]
    fn complete_current_picks_next() {
        let mut mgr = new_manager();
        mgr.enqueue(1);
        mgr.enqueue(2);
        let next = mgr.complete_current();
        assert_eq!(next, Some(2));
        assert_eq!(mgr.current_task(), Some(2));
    }

    #[test]
    fn complete_current_returns_none_when_empty() {
        let mut mgr = new_manager();
        mgr.enqueue(1);
        let result = mgr.complete_current();
        assert_eq!(result, None);
        assert_eq!(mgr.current_task(), None);
        assert!(mgr.is_idle());
    }

    #[test]
    fn cancel_running_task() {
        let mut mgr = new_manager();
        mgr.enqueue(1);
        let token = CancellationToken::new();
        mgr.set_cancellation_token(1, token.clone());
        let action = mgr.cancel_task(1);
        assert!(matches!(action, CancelAction::Running));
        assert!(token.is_cancelled());
    }

    #[test]
    fn cancel_queued_task() {
        let mut mgr = new_manager();
        mgr.enqueue(1);
        mgr.enqueue(2);
        let action = mgr.cancel_task(2);
        assert!(matches!(action, CancelAction::RemovedFromQueue));
    }

    #[test]
    fn cancel_nonexistent_task() {
        let mut mgr = new_manager();
        let action = mgr.cancel_task(99);
        assert!(matches!(action, CancelAction::NotFound));
    }

    #[test]
    fn enqueue_and_complete_dispatch_to_runner() {
        let (tx, _rx) = broadcast::channel(16);
        let (runner_tx, runner_rx) = std::sync::mpsc::channel();
        let mut mgr = CrawlTaskManager::new(tx);
        mgr.set_runner_tx(runner_tx);

        assert!(mgr.enqueue(1));
        assert!(matches!(
            runner_rx.recv().expect("runner command"),
            RunnerCommand::Execute { task_id: 1 }
        ));

        assert!(!mgr.enqueue(2));
        assert_eq!(mgr.complete_current(), Some(2));
        assert!(matches!(
            runner_rx.recv().expect("runner command"),
            RunnerCommand::Execute { task_id: 2 }
        ));
    }

    #[test]
    fn is_idle_when_empty() {
        let mgr = new_manager();
        assert!(mgr.is_idle());
    }

    #[test]
    fn reconcile_startup_queues_tasks() {
        let mut mgr = new_manager();
        mgr.reconcile_startup(vec![10, 20, 30]);
        assert_eq!(mgr.current_task(), Some(10));
    }

    #[test]
    fn emit_event_broadcasts() {
        let (tx, mut rx) = broadcast::channel(16);
        let mgr = CrawlTaskManager::new(tx);
        mgr.emit_event(SseEvent::TaskStarted {
            task_id: 1,
            user_id: "user1".to_owned(),
            task_type: "batch".to_owned(),
        });
        let event = rx.try_recv().expect("event should be available");
        assert!(matches!(event, SseEvent::TaskStarted { task_id: 1, .. }));
    }
}
