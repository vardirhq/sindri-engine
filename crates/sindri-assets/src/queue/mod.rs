use std::collections::BTreeSet;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

#[cfg(target_arch = "wasm32")]
use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};
#[cfg(not(target_arch = "wasm32"))]
use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
};

#[cfg(target_arch = "wasm32")]
use futures::{Stream, stream::FuturesUnordered};
use sindri_core::{AssetHandle, AssetId};
use thiserror::Error;

use crate::{AssetBytes, AssetSource, AssetSourceError};

/// Identity carried through an asynchronous load operation.
///
/// The generation prevents a completion for an expired handle from being
/// mistaken for a later request of the same logical asset ID.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AssetLoadRequest {
    id: AssetId,
    generation: u64,
}

impl AssetLoadRequest {
    pub fn new<T>(handle: &AssetHandle<T>) -> Self {
        Self {
            id: handle.id().clone(),
            generation: handle.generation(),
        }
    }

    pub fn id(&self) -> &AssetId {
        &self.id
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn matches<T>(&self, handle: &AssetHandle<T>) -> bool {
        self.id == *handle.id() && self.generation == handle.generation()
    }
}

#[derive(Debug)]
pub struct AssetLoadCompletion {
    request: AssetLoadRequest,
    result: Result<AssetBytes, AssetSourceError>,
}

impl AssetLoadCompletion {
    pub fn request(&self) -> &AssetLoadRequest {
        &self.request
    }

    pub fn result(&self) -> Result<&AssetBytes, &AssetSourceError> {
        self.result.as_ref()
    }

    pub fn into_result(self) -> Result<AssetBytes, AssetSourceError> {
        self.result
    }

    pub fn into_parts(self) -> (AssetLoadRequest, Result<AssetBytes, AssetSourceError>) {
        (self.request, self.result)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetLoadQueueConfig {
    pub max_concurrent: usize,
    /// How many requests may wait at once; [`usize::MAX`] for no limit.
    pub capacity: usize,
}

impl AssetLoadQueueConfig {
    pub const fn new(max_concurrent: usize, capacity: usize) -> Self {
        Self {
            max_concurrent,
            capacity,
        }
    }

    /// A queue that takes every request it is given.
    ///
    /// For a caller that asks for a known set of files, such as an editor
    /// opening a project: refusing some of them only means they arrive late,
    /// or not at all, and nothing is saved by it. Waiting requests cost one
    /// entry each; nothing is reserved up front.
    pub const fn unbounded(max_concurrent: usize) -> Self {
        Self::new(max_concurrent, usize::MAX)
    }
}

impl Default for AssetLoadQueueConfig {
    fn default() -> Self {
        Self::new(2, 64)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum AssetLoadQueueCreateError {
    #[error("asset load queue concurrency must be greater than zero")]
    ZeroConcurrency,
    #[error("asset load queue capacity must be greater than zero")]
    ZeroCapacity,
    #[cfg(not(target_arch = "wasm32"))]
    #[error("failed to spawn asset I/O worker: {0}")]
    WorkerSpawn(String),
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum AssetLoadQueueError {
    #[error("asset request '{0}' is already queued or loading")]
    Duplicate(AssetId),
    #[error("asset load queue is full (capacity {capacity})")]
    Full { capacity: usize },
    #[error("asset load queue workers have stopped")]
    Closed,
}

/// Cross-platform queue for loading undecoded asset bytes, bounded or not.
///
/// Native builds create and poll source futures on dedicated I/O workers, so a
/// blocking filesystem source never runs on the frame thread. WebAssembly
/// builds retain futures locally and advance them from [`Self::drain`], allowing
/// the browser Fetch API to remain genuinely asynchronous.
pub struct AssetLoadQueue {
    config: AssetLoadQueueConfig,
    outstanding: BTreeSet<AssetLoadRequest>,
    #[cfg(not(target_arch = "wasm32"))]
    task_sender: Option<Sender<AssetLoadRequest>>,
    #[cfg(not(target_arch = "wasm32"))]
    completion_receiver: Receiver<AssetLoadCompletion>,
    #[cfg(not(target_arch = "wasm32"))]
    workers: Vec<JoinHandle<()>>,
    #[cfg(target_arch = "wasm32")]
    source: Rc<dyn AssetSource>,
    #[cfg(target_arch = "wasm32")]
    waiting: VecDeque<AssetLoadRequest>,
    #[cfg(target_arch = "wasm32")]
    active: FuturesUnordered<LocalLoadFuture>,
}

#[cfg(target_arch = "wasm32")]
type LocalLoadFuture = Pin<Box<dyn Future<Output = AssetLoadCompletion> + 'static>>;

impl AssetLoadQueue {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new<S>(
        source: S,
        config: AssetLoadQueueConfig,
    ) -> Result<Self, AssetLoadQueueCreateError>
    where
        S: AssetSource + Send + Sync + 'static,
    {
        validate_config(config)?;

        let source: Arc<dyn AssetSource + Send + Sync> = Arc::new(source);
        // Unbounded, because the capacity is enforced on `outstanding` before a
        // request is sent; a bounded channel would reserve room for the whole
        // capacity up front, which an unbounded queue cannot give.
        let (task_sender, task_receiver) = mpsc::channel::<AssetLoadRequest>();
        let task_receiver = Arc::new(Mutex::new(task_receiver));
        let (completion_sender, completion_receiver) = mpsc::channel();
        let mut workers = Vec::with_capacity(config.max_concurrent);

        for index in 0..config.max_concurrent {
            let source = Arc::clone(&source);
            let task_receiver = Arc::clone(&task_receiver);
            let completion_sender = completion_sender.clone();
            let spawn = thread::Builder::new()
                .name(format!("sindri-asset-io-{index}"))
                .spawn(move || {
                    loop {
                        let request = {
                            let receiver = task_receiver
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            receiver.recv()
                        };
                        let Ok(request) = request else {
                            break;
                        };
                        let result = futures::executor::block_on(source.load(request.id()));
                        if completion_sender
                            .send(AssetLoadCompletion { request, result })
                            .is_err()
                        {
                            break;
                        }
                    }
                });

            match spawn {
                Ok(worker) => workers.push(worker),
                Err(error) => {
                    drop(task_sender);
                    for worker in workers {
                        let _ = worker.join();
                    }
                    return Err(AssetLoadQueueCreateError::WorkerSpawn(error.to_string()));
                }
            }
        }

        Ok(Self {
            config,
            outstanding: BTreeSet::new(),
            task_sender: Some(task_sender),
            completion_receiver,
            workers,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new<S>(
        source: S,
        config: AssetLoadQueueConfig,
    ) -> Result<Self, AssetLoadQueueCreateError>
    where
        S: AssetSource + 'static,
    {
        validate_config(config)?;
        Ok(Self {
            config,
            outstanding: BTreeSet::new(),
            source: Rc::new(source),
            waiting: VecDeque::new(),
            active: FuturesUnordered::new(),
        })
    }

    pub fn enqueue(&mut self, request: AssetLoadRequest) -> Result<(), AssetLoadQueueError> {
        if self.outstanding.contains(&request) {
            return Err(AssetLoadQueueError::Duplicate(request.id().clone()));
        }
        if self.outstanding.len() >= self.config.capacity {
            return Err(AssetLoadQueueError::Full {
                capacity: self.config.capacity,
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let sender = self
                .task_sender
                .as_ref()
                .ok_or(AssetLoadQueueError::Closed)?;
            sender
                .send(request.clone())
                .map_err(|_| AssetLoadQueueError::Closed)?;
        }

        #[cfg(target_arch = "wasm32")]
        self.waiting.push_back(request.clone());

        self.outstanding.insert(request);
        Ok(())
    }

    pub fn drain(&mut self) -> Vec<AssetLoadCompletion> {
        #[cfg(not(target_arch = "wasm32"))]
        let completions = self.completion_receiver.try_iter().collect::<Vec<_>>();

        #[cfg(target_arch = "wasm32")]
        let completions = {
            self.start_waiting();
            let waker = Waker::noop();
            let mut context = Context::from_waker(waker);
            let mut completions = Vec::new();
            while let Poll::Ready(Some(completion)) =
                Pin::new(&mut self.active).poll_next(&mut context)
            {
                completions.push(completion);
                self.start_waiting();
            }
            completions
        };

        for completion in &completions {
            self.outstanding.remove(completion.request());
        }
        completions
    }

    pub fn outstanding(&self) -> usize {
        self.outstanding.len()
    }

    pub fn is_empty(&self) -> bool {
        self.outstanding.is_empty()
    }

    pub const fn capacity(&self) -> usize {
        self.config.capacity
    }

    #[cfg(target_arch = "wasm32")]
    fn start_waiting(&mut self) {
        while self.active.len() < self.config.max_concurrent {
            let Some(request) = self.waiting.pop_front() else {
                break;
            };
            let source = Rc::clone(&self.source);
            self.active.push(Box::pin(async move {
                let result = source.load(request.id()).await;
                AssetLoadCompletion { request, result }
            }));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for AssetLoadQueue {
    fn drop(&mut self) {
        self.task_sender.take();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn validate_config(config: AssetLoadQueueConfig) -> Result<(), AssetLoadQueueCreateError> {
    if config.max_concurrent == 0 {
        return Err(AssetLoadQueueCreateError::ZeroConcurrency);
    }
    if config.capacity == 0 {
        return Err(AssetLoadQueueCreateError::ZeroCapacity);
    }
    Ok(())
}
