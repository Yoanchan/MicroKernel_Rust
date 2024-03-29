// use alloc::boxed::Box;
// use core::sync::atomic::{AtomicU64, Ordering};
// use core::task::{Context, Poll};
// use core::{future::Future, pin::Pin};

// pub mod executor;
// pub mod keyboard;

// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// struct TaskId(u64);

// impl TaskId {
//     fn new() -> Self {
//         static NEXT_ID: AtomicU64 = AtomicU64::new(0);
//         TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
//     }
// }

// pub struct Task {
//     id: TaskId,
//     future: Pin<Box<dyn Future<Output = ()>>>,
// }

// impl Task {
//     pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
//         Task {
//             id: TaskId::new(),
//             future: Box::pin(future),
//         }
//     }
//     fn poll(&mut self, context: &mut Context) -> Poll<()> {
//         self.future.as_mut().poll(context)
//     }

//     fn id(&self) -> TaskId {
//         use core::ops::Deref;

//         let addr = Pin::deref(&self.future) as *const _ as *const () as u64;
//         TaskId(addr)
//     }
// }

use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};
use core::task::{Context, Poll};
use core::{future::Future, pin::Pin};

pub mod scheduler;
pub mod yields;

pub use self::yields::yield_init;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

pub trait TaskFuture {
    fn id(&self) -> TaskId;
    fn poll(&mut self, context: &mut Context) -> Poll<()>;
}

pub struct Task {
    id: TaskId,
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static) -> Self {
        Task {
            id: TaskId::new(),
            future: Box::pin(future),
        }
    }
}

impl TaskFuture for Task {
    fn id(&self) -> TaskId {
        self.id
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Medium,
    High,
}

pub struct PriorityTask {
    priority: Priority,
    inner: Task,
}

impl PriorityTask {
    pub fn new(priority: Priority, future: impl Future<Output = ()> + 'static) -> Self {
        PriorityTask {
            priority,
            inner: Task::new(future),
        }
    }

    pub fn priority(&self) -> Priority {
        self.priority
    }
}

impl TaskFuture for PriorityTask {
    fn id(&self) -> TaskId {
        self.inner.id
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.inner.future.as_mut().poll(context)
    }
}
