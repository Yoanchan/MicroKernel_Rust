use core::task::Waker;

use alloc::{sync::Arc, task::Wake};
use crossbeam_queue::ArrayQueue;

use super::{TaskFuture, TaskId};

pub mod priority;

#[derive(Debug)]
pub enum Error {
    DuplicateId,
    TaskQueueFull,
    UnknownId,
}

pub trait Scheduler<T: TaskFuture> {
    fn run(&mut self) -> !;
    fn spawn(&mut self, task: T) -> Result<(), Error>;
    fn kill(&mut self, task_id: TaskId) -> Result<(), Error>;
}

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task_queue full");
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
