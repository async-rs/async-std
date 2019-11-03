use std::cell::UnsafeCell;
use std::fmt;
use std::future::Future;
use std::isize;
use std::marker::PhantomData;
use std::mem;
use std::pin::Pin;
use std::process;
use std::ptr;
use std::sync::atomic::{self, AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use crossbeam_utils::{Backoff, CachePadded};

use crate::stream::Stream;
use crate::sync::WakerSet;

/// Creates a bounded multi-producer multi-consumer channel.
///
/// This channel has a buffer that can hold at most `cap` messages at a time.
///
/// Senders and receivers can be cloned. When all senders associated with a channel get dropped, it
/// becomes closed. Receive operations on a closed and empty channel return `None` instead of
/// trying to await a message.
///
/// # Panics
///
/// If `cap` is zero, this function will panic.
///
/// # Examples
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use std::time::Duration;
///
/// use async_std::sync::channel;
/// use async_std::task;
///
/// let (s, r) = channel(1);
///
/// // This call returns immediately because there is enough space in the channel.
/// s.send(1).await;
///
/// task::spawn(async move {
///     // This call will have to wait because the channel is full.
///     // It will be able to complete only after the first message is received.
///     s.send(2).await;
/// });
///
/// task::sleep(Duration::from_secs(1)).await;
/// assert_eq!(r.recv().await, Some(1));
/// assert_eq!(r.recv().await, Some(2));
/// #
/// # })
/// ```
#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
pub fn channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let channel = Arc::new(Channel::with_capacity(cap));
    let s = Sender {
        channel: channel.clone(),
    };
    let r = Receiver {
        channel,
        opt_key: None,
    };
    (s, r)
}

/// The sending side of a channel.
///
/// # Examples
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use async_std::sync::channel;
/// use async_std::task;
///
/// let (s1, r) = channel(100);
/// let s2 = s1.clone();
///
/// task::spawn(async move { s1.send(1).await });
/// task::spawn(async move { s2.send(2).await });
///
/// let msg1 = r.recv().await.unwrap();
/// let msg2 = r.recv().await.unwrap();
///
/// assert_eq!(msg1 + msg2, 3);
/// #
/// # })
/// ```
#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
pub struct Sender<T> {
    /// The inner channel.
    channel: Arc<Channel<T>>,
}

impl<T> Sender<T> {
    /// Sends a message into the channel.
    ///
    /// If the channel is full, this method will wait until there is space in the channel.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::channel;
    /// use async_std::task;
    ///
    /// let (s, r) = channel(1);
    ///
    /// task::spawn(async move {
    ///     s.send(1).await;
    ///     s.send(2).await;
    /// });
    ///
    /// assert_eq!(r.recv().await, Some(1));
    /// assert_eq!(r.recv().await, Some(2));
    /// assert_eq!(r.recv().await, None);
    /// #
    /// # })
    /// ```
    pub async fn send(&self, msg: T) {
        struct SendFuture<'a, T> {
            channel: &'a Channel<T>,
            msg: Option<T>,
            opt_key: Option<usize>,
        }

        impl<T> Unpin for SendFuture<'_, T> {}

        impl<T> Future for SendFuture<'_, T> {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let msg = self.msg.take().unwrap();

                // Try sending the message.
                let poll = match self.channel.try_send(msg) {
                    Ok(()) => Poll::Ready(()),
                    Err(TrySendError::Disconnected(msg)) => {
                        self.msg = Some(msg);
                        Poll::Pending
                    }
                    Err(TrySendError::Full(msg)) => {
                        // Insert this send operation.
                        match self.opt_key {
                            None => self.opt_key = Some(self.channel.send_wakers.insert(cx)),
                            Some(key) => self.channel.send_wakers.update(key, cx),
                        }

                        // Try sending the message again.
                        match self.channel.try_send(msg) {
                            Ok(()) => Poll::Ready(()),
                            Err(TrySendError::Disconnected(msg)) | Err(TrySendError::Full(msg)) => {
                                self.msg = Some(msg);
                                Poll::Pending
                            }
                        }
                    }
                };

                if poll.is_ready() {
                    // If the current task is in the set, remove it.
                    if let Some(key) = self.opt_key.take() {
                        self.channel.send_wakers.complete(key);
                    }
                }

                poll
            }
        }

        impl<T> Drop for SendFuture<'_, T> {
            fn drop(&mut self) {
                // If the current task is still in the set, that means it is being cancelled now.
                // Wake up another task instead.
                if let Some(key) = self.opt_key {
                    self.channel.send_wakers.cancel(key);
                }
            }
        }

        SendFuture {
            channel: &self.channel,
            msg: Some(msg),
            opt_key: None,
        }
        .await
    }

    /// Returns the channel capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::sync::channel;
    ///
    /// let (s, _) = channel::<i32>(5);
    /// assert_eq!(s.capacity(), 5);
    /// ```
    pub fn capacity(&self) -> usize {
        self.channel.cap
    }

    /// Returns `true` if the channel is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::channel;
    ///
    /// let (s, r) = channel(1);
    ///
    /// assert!(s.is_empty());
    /// s.send(0).await;
    /// assert!(!s.is_empty());
    /// #
    /// # })
    /// ```
    pub fn is_empty(&self) -> bool {
        self.channel.is_empty()
    }

    /// Returns `true` if the channel is full.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::channel;
    ///
    /// let (s, r) = channel(1);
    ///
    /// assert!(!s.is_full());
    /// s.send(0).await;
    /// assert!(s.is_full());
    /// #
    /// # })
    /// ```
    pub fn is_full(&self) -> bool {
        self.channel.is_full()
    }

    /// Returns the number of messages in the channel.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::channel;
    ///
    /// let (s, r) = channel(2);
    /// assert_eq!(s.len(), 0);
    ///
    /// s.send(1).await;
    /// s.send(2).await;
    /// assert_eq!(s.len(), 2);
    /// #
    /// # })
    /// ```
    pub fn len(&self) -> usize {
        self.channel.len()
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        // Decrement the sender count and disconnect the channel if it drops down to zero.
        if self.channel.sender_count.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.channel.disconnect();
        }
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Sender<T> {
        let count = self.channel.sender_count.fetch_add(1, Ordering::Relaxed);

        // Make sure the count never overflows, even if lots of sender clones are leaked.
        if count > isize::MAX as usize {
            process::abort();
        }

        Sender {
            channel: self.channel.clone(),
        }
    }
}

impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Sender { .. }")
    }
}

/// The receiving side of a channel.
///
/// This type implements the [`Stream`] trait, which means it can act as an asynchronous iterator.
///
/// [`Stream`]: ../stream/trait.Stream.html
///
/// # Examples
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use std::time::Duration;
///
/// use async_std::sync::channel;
/// use async_std::task;
///
/// let (s, r) = channel(100);
///
/// task::spawn(async move {
///     s.send(1).await;
///     task::sleep(Duration::from_secs(1)).await;
///     s.send(2).await;
/// });
///
/// assert_eq!(r.recv().await, Some(1)); // Received immediately.
/// assert_eq!(r.recv().await, Some(2)); // Received after 1 second.
/// #
/// # })
/// ```
#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
pub struct Receiver<T> {
    /// The inner channel.
    channel: Arc<Channel<T>>,

    /// The key for this receiver in the `channel.stream_wakers` set.
    opt_key: Option<usize>,
}

impl<T> Receiver<T> {
    /// Receives a message from the channel.
    ///
    /// If the channel is empty and still has senders, this method will wait until a message is
    /// sent into the channel or until all senders get dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::channel;
    /// use async_std::task;
    ///
    /// let (s, r) = channel(1);
    ///
    /// task::spawn(async move {
    ///     s.send(1).await;
    ///     s.send(2).await;
    /// });
    ///
    /// assert_eq!(r.recv().await, Some(1));
    /// assert_eq!(r.recv().await, Some(2));
    /// assert_eq!(r.recv().await, None);
    /// #
    /// # })
    /// ```
    pub async fn recv(&self) -> Option<T> {
        struct RecvFuture<'a, T> {
            channel: &'a Channel<T>,
            opt_key: Option<usize>,
        }

        impl<T> Future for RecvFuture<'_, T> {
            type Output = Option<T>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                poll_recv(
                    &self.channel,
                    &self.channel.recv_wakers,
                    &mut self.opt_key,
                    cx,
                )
            }
        }

        impl<T> Drop for RecvFuture<'_, T> {
            fn drop(&mut self) {
                // If the current task is still in the set, that means it is being cancelled now.
                if let Some(key) = self.opt_key {
                    self.channel.recv_wakers.cancel(key);
                }
            }
        }

        RecvFuture {
            channel: &self.channel,
            opt_key: None,
        }
        .await
    }

    /// Returns the channel capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::sync::channel;
    ///
    /// let (_, r) = channel::<i32>(5);
    /// assert_eq!(r.capacity(), 5);
    /// ```
    pub fn capacity(&self) -> usize {
        self.channel.cap
    }

    /// Returns `true` if the channel is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::channel;
    ///
    /// let (s, r) = channel(1);
    ///
    /// assert!(r.is_empty());
    /// s.send(0).await;
    /// assert!(!r.is_empty());
    /// #
    /// # })
    /// ```
    pub fn is_empty(&self) -> bool {
        self.channel.is_empty()
    }

    /// Returns `true` if the channel is full.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::channel;
    ///
    /// let (s, r) = channel(1);
    ///
    /// assert!(!r.is_full());
    /// s.send(0).await;
    /// assert!(r.is_full());
    /// #
    /// # })
    /// ```
    pub fn is_full(&self) -> bool {
        self.channel.is_full()
    }

    /// Returns the number of messages in the channel.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::channel;
    ///
    /// let (s, r) = channel(2);
    /// assert_eq!(r.len(), 0);
    ///
    /// s.send(1).await;
    /// s.send(2).await;
    /// assert_eq!(r.len(), 2);
    /// #
    /// # })
    /// ```
    pub fn len(&self) -> usize {
        self.channel.len()
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        // If the current task is still in the stream set, that means it is being cancelled now.
        if let Some(key) = self.opt_key {
            self.channel.stream_wakers.cancel(key);
        }

        // Decrement the receiver count and disconnect the channel if it drops down to zero.
        if self.channel.receiver_count.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.channel.disconnect();
        }
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Receiver<T> {
        let count = self.channel.receiver_count.fetch_add(1, Ordering::Relaxed);

        // Make sure the count never overflows, even if lots of receiver clones are leaked.
        if count > isize::MAX as usize {
            process::abort();
        }

        Receiver {
            channel: self.channel.clone(),
            opt_key: None,
        }
    }
}

impl<T> Stream for Receiver<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        poll_recv(
            &this.channel,
            &this.channel.stream_wakers,
            &mut this.opt_key,
            cx,
        )
    }
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Receiver { .. }")
    }
}

/// Polls a receive operation on a channel.
///
/// If the receive operation is blocked, the current task will be inserted into `wakers` and its
/// associated key will then be stored in `opt_key`.
fn poll_recv<T>(
    channel: &Channel<T>,
    wakers: &WakerSet,
    opt_key: &mut Option<usize>,
    cx: &mut Context<'_>,
) -> Poll<Option<T>> {
    // Try receiving a message.
    let poll = match channel.try_recv() {
        Ok(msg) => Poll::Ready(Some(msg)),
        Err(TryRecvError::Disconnected) => Poll::Ready(None),
        Err(TryRecvError::Empty) => {
            // Insert this receive operation.
            match *opt_key {
                None => *opt_key = Some(wakers.insert(cx)),
                Some(key) => wakers.update(key, cx),
            }

            // Try receiving a message again.
            match channel.try_recv() {
                Ok(msg) => Poll::Ready(Some(msg)),
                Err(TryRecvError::Disconnected) => Poll::Ready(None),
                Err(TryRecvError::Empty) => Poll::Pending,
            }
        }
    };

    if poll.is_ready() {
        // If the current task is in the set, remove it.
        if let Some(key) = opt_key.take() {
            wakers.complete(key);
        }
    }

    poll
}

/// A slot in a channel.
struct Slot<T> {
    /// The current stamp.
    stamp: AtomicUsize,

    /// The message in this slot.
    msg: UnsafeCell<T>,
}

/// Bounded channel based on a preallocated array.
struct Channel<T> {
    /// The head of the channel.
    ///
    /// This value is a "stamp" consisting of an index into the buffer, a mark bit, and a lap, but
    /// packed into a single `usize`. The lower bits represent the index, while the upper bits
    /// represent the lap. The mark bit in the head is always zero.
    ///
    /// Messages are popped from the head of the channel.
    head: CachePadded<AtomicUsize>,

    /// The tail of the channel.
    ///
    /// This value is a "stamp" consisting of an index into the buffer, a mark bit, and a lap, but
    /// packed into a single `usize`. The lower bits represent the index, while the upper bits
    /// represent the lap. The mark bit indicates that the channel is disconnected.
    ///
    /// Messages are pushed into the tail of the channel.
    tail: CachePadded<AtomicUsize>,

    /// The buffer holding slots.
    buffer: *mut Slot<T>,

    /// The channel capacity.
    cap: usize,

    /// A stamp with the value of `{ lap: 1, mark: 0, index: 0 }`.
    one_lap: usize,

    /// If this bit is set in the tail, that means either all senders were dropped or all receivers
    /// were dropped.
    mark_bit: usize,

    /// Send operations waiting while the channel is full.
    send_wakers: WakerSet,

    /// Receive operations waiting while the channel is empty and not disconnected.
    recv_wakers: WakerSet,

    /// Streams waiting while the channel is empty and not disconnected.
    stream_wakers: WakerSet,

    /// The number of currently active `Sender`s.
    sender_count: AtomicUsize,

    /// The number of currently active `Receivers`s.
    receiver_count: AtomicUsize,

    /// Indicates that dropping a `Channel<T>` may drop values of type `T`.
    _marker: PhantomData<T>,
}

unsafe impl<T: Send> Send for Channel<T> {}
unsafe impl<T: Send> Sync for Channel<T> {}
impl<T> Unpin for Channel<T> {}

impl<T> Channel<T> {
    /// Creates a bounded channel of capacity `cap`.
    fn with_capacity(cap: usize) -> Self {
        assert!(cap > 0, "capacity must be positive");

        // Compute constants `mark_bit` and `one_lap`.
        let mark_bit = (cap + 1).next_power_of_two();
        let one_lap = mark_bit * 2;

        // Head is initialized to `{ lap: 0, mark: 0, index: 0 }`.
        let head = 0;
        // Tail is initialized to `{ lap: 0, mark: 0, index: 0 }`.
        let tail = 0;

        // Allocate a buffer of `cap` slots.
        let buffer = {
            let mut v = Vec::<Slot<T>>::with_capacity(cap);
            let ptr = v.as_mut_ptr();
            mem::forget(v);
            ptr
        };

        // Initialize stamps in the slots.
        for i in 0..cap {
            unsafe {
                // Set the stamp to `{ lap: 0, mark: 0, index: i }`.
                let slot = buffer.add(i);
                ptr::write(&mut (*slot).stamp, AtomicUsize::new(i));
            }
        }

        Channel {
            buffer,
            cap,
            one_lap,
            mark_bit,
            head: CachePadded::new(AtomicUsize::new(head)),
            tail: CachePadded::new(AtomicUsize::new(tail)),
            send_wakers: WakerSet::new(),
            recv_wakers: WakerSet::new(),
            stream_wakers: WakerSet::new(),
            sender_count: AtomicUsize::new(1),
            receiver_count: AtomicUsize::new(1),
            _marker: PhantomData,
        }
    }

    /// Attempts to send a message.
    fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
        let backoff = Backoff::new();
        let mut tail = self.tail.load(Ordering::Relaxed);

        loop {
            // Deconstruct the tail.
            let index = tail & (self.mark_bit - 1);
            let lap = tail & !(self.one_lap - 1);

            // Inspect the corresponding slot.
            let slot = unsafe { &*self.buffer.add(index) };
            let stamp = slot.stamp.load(Ordering::Acquire);

            // If the tail and the stamp match, we may attempt to push.
            if tail == stamp {
                let new_tail = if index + 1 < self.cap {
                    // Same lap, incremented index.
                    // Set to `{ lap: lap, mark: 0, index: index + 1 }`.
                    tail + 1
                } else {
                    // One lap forward, index wraps around to zero.
                    // Set to `{ lap: lap.wrapping_add(1), mark: 0, index: 0 }`.
                    lap.wrapping_add(self.one_lap)
                };

                // Try moving the tail.
                match self.tail.compare_exchange_weak(
                    tail,
                    new_tail,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        // Write the message into the slot and update the stamp.
                        unsafe { slot.msg.get().write(msg) };
                        let stamp = tail + 1;
                        slot.stamp.store(stamp, Ordering::Release);

                        // Wake a blocked receive operation.
                        self.recv_wakers.notify_one();

                        // Wake all blocked streams.
                        self.stream_wakers.notify_all();

                        return Ok(());
                    }
                    Err(t) => {
                        tail = t;
                        backoff.spin();
                    }
                }
            } else if stamp.wrapping_add(self.one_lap) == tail + 1 {
                atomic::fence(Ordering::SeqCst);
                let head = self.head.load(Ordering::Relaxed);

                // If the head lags one lap behind the tail as well...
                if head.wrapping_add(self.one_lap) == tail {
                    // ...then the channel is full.

                    // Check if the channel is disconnected.
                    if tail & self.mark_bit != 0 {
                        return Err(TrySendError::Disconnected(msg));
                    } else {
                        return Err(TrySendError::Full(msg));
                    }
                }

                backoff.spin();
                tail = self.tail.load(Ordering::Relaxed);
            } else {
                // Snooze because we need to wait for the stamp to get updated.
                backoff.snooze();
                tail = self.tail.load(Ordering::Relaxed);
            }
        }
    }

    /// Attempts to receive a message.
    fn try_recv(&self) -> Result<T, TryRecvError> {
        let backoff = Backoff::new();
        let mut head = self.head.load(Ordering::Relaxed);

        loop {
            // Deconstruct the head.
            let index = head & (self.mark_bit - 1);
            let lap = head & !(self.one_lap - 1);

            // Inspect the corresponding slot.
            let slot = unsafe { &*self.buffer.add(index) };
            let stamp = slot.stamp.load(Ordering::Acquire);

            // If the the stamp is ahead of the head by 1, we may attempt to pop.
            if head + 1 == stamp {
                let new = if index + 1 < self.cap {
                    // Same lap, incremented index.
                    // Set to `{ lap: lap, mark: 0, index: index + 1 }`.
                    head + 1
                } else {
                    // One lap forward, index wraps around to zero.
                    // Set to `{ lap: lap.wrapping_add(1), mark: 0, index: 0 }`.
                    lap.wrapping_add(self.one_lap)
                };

                // Try moving the head.
                match self.head.compare_exchange_weak(
                    head,
                    new,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        // Read the message from the slot and update the stamp.
                        let msg = unsafe { slot.msg.get().read() };
                        let stamp = head.wrapping_add(self.one_lap);
                        slot.stamp.store(stamp, Ordering::Release);

                        // Wake a blocked send operation.
                        self.send_wakers.notify_one();

                        return Ok(msg);
                    }
                    Err(h) => {
                        head = h;
                        backoff.spin();
                    }
                }
            } else if stamp == head {
                atomic::fence(Ordering::SeqCst);
                let tail = self.tail.load(Ordering::Relaxed);

                // If the tail equals the head, that means the channel is empty.
                if (tail & !self.mark_bit) == head {
                    // If the channel is disconnected...
                    if tail & self.mark_bit != 0 {
                        return Err(TryRecvError::Disconnected);
                    } else {
                        // Otherwise, the receive operation is not ready.
                        return Err(TryRecvError::Empty);
                    }
                }

                backoff.spin();
                head = self.head.load(Ordering::Relaxed);
            } else {
                // Snooze because we need to wait for the stamp to get updated.
                backoff.snooze();
                head = self.head.load(Ordering::Relaxed);
            }
        }
    }

    /// Returns the current number of messages inside the channel.
    fn len(&self) -> usize {
        loop {
            // Load the tail, then load the head.
            let tail = self.tail.load(Ordering::SeqCst);
            let head = self.head.load(Ordering::SeqCst);

            // If the tail didn't change, we've got consistent values to work with.
            if self.tail.load(Ordering::SeqCst) == tail {
                let hix = head & (self.mark_bit - 1);
                let tix = tail & (self.mark_bit - 1);

                return if hix < tix {
                    tix - hix
                } else if hix > tix {
                    self.cap - hix + tix
                } else if (tail & !self.mark_bit) == head {
                    0
                } else {
                    self.cap
                };
            }
        }
    }

    /// Returns `true` if the channel is empty.
    fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);

        // Is the tail equal to the head?
        //
        // Note: If the head changes just before we load the tail, that means there was a moment
        // when the channel was not empty, so it is safe to just return `false`.
        (tail & !self.mark_bit) == head
    }

    /// Returns `true` if the channel is full.
    fn is_full(&self) -> bool {
        let tail = self.tail.load(Ordering::SeqCst);
        let head = self.head.load(Ordering::SeqCst);

        // Is the head lagging one lap behind tail?
        //
        // Note: If the tail changes just before we load the head, that means there was a moment
        // when the channel was not full, so it is safe to just return `false`.
        head.wrapping_add(self.one_lap) == tail & !self.mark_bit
    }

    /// Disconnects the channel and wakes up all blocked operations.
    fn disconnect(&self) {
        let tail = self.tail.fetch_or(self.mark_bit, Ordering::SeqCst);

        if tail & self.mark_bit == 0 {
            // Notify everyone blocked on this channel.
            self.send_wakers.notify_all();
            self.recv_wakers.notify_all();
            self.stream_wakers.notify_all();
        }
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        // Get the index of the head.
        let hix = self.head.load(Ordering::Relaxed) & (self.mark_bit - 1);

        // Loop over all slots that hold a message and drop them.
        for i in 0..self.len() {
            // Compute the index of the next slot holding a message.
            let index = if hix + i < self.cap {
                hix + i
            } else {
                hix + i - self.cap
            };

            unsafe {
                self.buffer.add(index).drop_in_place();
            }
        }

        // Finally, deallocate the buffer, but don't run any destructors.
        unsafe {
            Vec::from_raw_parts(self.buffer, 0, self.cap);
        }
    }
}

/// An error returned from the `try_send()` method.
enum TrySendError<T> {
    /// The channel is full but not disconnected.
    Full(T),

    /// The channel is full and disconnected.
    Disconnected(T),
}

/// An error returned from the `try_recv()` method.
enum TryRecvError {
    /// The channel is empty but not disconnected.
    Empty,

    /// The channel is empty and disconnected.
    Disconnected,
}
