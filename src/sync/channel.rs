use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::stream::Stream;
use crate::channel::{bounded, Sender as InnerSender, Receiver as InnerReceiver, SendError};

#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[doc(inline)]
pub use crate::channel::{RecvError, TryRecvError, TrySendError};

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
/// # fn main() -> Result<(), async_std::sync::RecvError> {
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
/// s.send(1usize).await;
///
/// task::spawn(async move {
///     // This call will have to wait because the channel is full.
///     // It will be able to complete only after the first message is received.
///     s.send(2).await;
/// });
///
/// task::sleep(Duration::from_secs(1)).await;
/// assert_eq!(r.recv().await?, 1);
/// assert_eq!(r.recv().await?, 2);
/// # Ok(())
/// #
/// # }) }
/// ```
#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
pub fn channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let (sender, receiver) = bounded(cap);
    
    let s = Sender {
        sender,
    };

    let r = Receiver {
        receiver,
    };

    (s, r)
}

/// The sending side of a channel.
///
/// This struct is created by the [`channel`] function. See its
/// documentation for more.
///
/// [`channel`]: fn.channel.html
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
    /// The sender.
    sender: InnerSender<T>,
}

impl<T> Sender<T> {
    /// Sends a message into the channel.
    ///
    /// If the channel is full, this method will wait until there is space in the channel.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), async_std::sync::RecvError> {
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
    /// assert_eq!(r.recv().await?, 1);
    /// assert_eq!(r.recv().await?, 2);
    /// assert!(r.recv().await.is_err());
    /// #
    /// # Ok(())
    /// # }) }
    /// ```
    pub async fn send(&self, msg: T) {
        struct SendFuture<'a, T> {
            inner: Pin<Box<dyn Future<Output = Result<(), SendError<T>>> + 'a>>,
        }

        unsafe impl<T: Send> Send for SendFuture<'_, T> {}
        
        impl<T> Unpin for SendFuture<'_, T> {}

        impl<T> Future for SendFuture<'_, T> {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                match Pin::new(&mut self.inner).poll(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Ok(())) => Poll::Ready(()),
                    Poll::Ready(Err(_)) => {
                        log::error!("attempted to send on dropped receiver");
                        // return pending to preserve old behaviour
                        Poll::Pending
                    }
                }
            }
        }       

        SendFuture {
            inner: Box::pin(self.sender.send(msg)),
        }
        .await
    }

    /// Attempts to send a message into the channel.
    ///
    /// If the channel is full, this method will return an error.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::channel;
    ///
    /// let (s, r) = channel(1);
    /// assert!(s.try_send(1).is_ok());
    /// assert!(s.try_send(2).is_err());
    /// #
    /// # })
    /// ```
    pub fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
        self.sender.try_send(msg)
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
        self.sender.capacity().unwrap()
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
        self.sender.is_empty()
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
        self.sender.is_full()
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
        self.sender.len()
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender {
            sender: self.sender.clone(),
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
/// This type receives messages by calling `recv`. But it also implements the [`Stream`] trait,
/// which means it can act as an asynchronous iterator. This struct is created by the [`channel`]
/// function. See its documentation for more.
///
/// [`channel`]: fn.channel.html
/// [`Stream`]: ../stream/trait.Stream.html
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), async_std::sync::RecvError> {
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
///     s.send(1usize).await;
///     task::sleep(Duration::from_secs(1)).await;
///     s.send(2).await;
/// });
///
/// assert_eq!(r.recv().await?, 1); // Received immediately.
/// assert_eq!(r.recv().await?, 2); // Received after 1 second.
/// #
/// # Ok(())
/// # }) }
/// ```
#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
pub struct Receiver<T> {
    /// The inner receiver.
    receiver: InnerReceiver<T>,
}

impl<T> Receiver<T> {
    /// Receives a message from the channel.
    ///
    /// If the channel is empty and still has senders, this method
    /// will wait until a message is sent into it. Once all senders
    /// have been dropped it will return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), async_std::sync::RecvError> {
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::channel;
    /// use async_std::task;
    ///
    /// let (s, r) = channel(1);
    ///
    /// task::spawn(async move {
    ///     s.send(1usize).await;
    ///     s.send(2).await;
    ///     // Then we drop the sender
    /// });
    ///
    /// assert_eq!(r.recv().await?, 1);
    /// assert_eq!(r.recv().await?, 2);
    /// assert!(r.recv().await.is_err());
    /// #
    /// # Ok(())
    /// # }) }
    /// ```
    pub async fn recv(&self) -> Result<T, RecvError> {
        self.receiver.recv().await
    }

    /// Attempts to receive a message from the channel.
    ///
    /// If the channel is empty, this method will return an error.
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
    /// s.send(1u8).await;
    ///
    /// assert!(r.try_recv().is_ok());
    /// assert!(r.try_recv().is_err());
    /// #
    /// # })
    /// ```
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.receiver.try_recv()
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
        self.receiver.capacity().unwrap()
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
        self.receiver.is_empty()
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
        self.receiver.is_full()
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
        self.receiver.len()
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Receiver {
            receiver: self.receiver.clone(),
        }
    }
}

impl<T> Stream for Receiver<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.receiver).poll_next(cx)
    }
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Receiver { .. }")
    }
}



