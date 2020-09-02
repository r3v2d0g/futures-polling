/**************************************************************************************************
 *                                                                                                *
 * This Source Code Form is subject to the terms of the Mozilla Public                            *
 * License, v. 2.0. If a copy of the MPL was not distributed with this                            *
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.                                       *
 *                                                                                                *
 **************************************************************************************************/

// =========================================== Imports ========================================== \\

use core::future::Future;
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll};

// ============================================ Types =========================================== \\

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug)]
pub enum Polling<Fut: Future> {
    Ready(Fut::Output),
    Pending(Fut),
    Done,
}

// ========================================= Interfaces ========================================= \\

/// Extension trait to easily convert a [`Future`] into a [`Polling`].
pub trait FuturePollingExt: Future {
    /// Returns [`Polling::Pending(self)`], consuming `self`.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use core::task::Poll;
    /// # use futures_lite::future;
    /// use futures_polling::FuturePollingExt;
    ///
    /// # future::block_on(async {
    /// #
    /// let mut polling = async { 42 }.polling();
    /// assert_eq!(polling.poll_once().await, Poll::Ready(42));
    /// #
    /// # });
    /// ```
    fn polling(self) -> Polling<Self>
    where
        Self: Sized,
    {
        Polling::Pending(self)
    }
}

impl<Fut: Future> FuturePollingExt for Fut {}

// =========================================== Polling ========================================== \\

impl<Fut: Future> Polling<Fut> {
    // ===================================== Destructors ==================================== \\

    /// Converts [`Polling::Ready(out)`] into [`Some(out)`], or returns [`None`] otherwise,
    /// consuming `self`.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.ready(), Some(42));
    ///
    /// let polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.ready(), None);
    ///
    /// let polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.ready(), None);
    /// ```
    ///
    /// [`Polling::Ready(out)`]: Polling::Ready
    /// [`Some(out)`]: core::option::Option::Some
    pub fn ready(self) -> Option<Fut::Output> {
        if let Polling::Ready(out) = self {
            Some(out)
        } else {
            None
        }
    }

    /// Converts [`Polling::Pending(fut)`] into [`Some(fut)`], or returns [`None`] otherwise,
    /// consuming `self`.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.pending().is_some(), false);
    ///
    /// let polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// # future::block_on(async {
    /// assert_eq!(polling.pending().unwrap().await, 42);
    /// # });
    ///
    /// let polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.pending().is_some(), false);
    /// ```
    ///
    /// [`Polling::Pending(fut)`]: Polling::Pending
    /// [`Some(fut)`]: core::option::Option::Some
    pub fn pending(self) -> Option<Fut> {
        if let Polling::Pending(fut) = self {
            Some(fut)
        } else {
            None
        }
    }

    /// Converts [`Polling::Ready(out)`] into [`Poll::Ready(out)`], or returns [`Poll::Pending`]
    /// otherwise, consuming `self`.
    ///
    /// Note that [`Polling::Done`] will also be converted to [`Poll::Pending`].
    ///
    /// ## Example
    ///
    /// ```rust
    /// use core::task::Poll;
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.into_poll(), Poll::Ready(42));
    ///
    /// let polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.into_poll(), Poll::Pending);
    ///
    /// let polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.into_poll(), Poll::Pending);
    /// ```
    ///
    /// [`Polling::Ready(out)`]: Polling::Ready
    /// [`Poll::Ready(out)`]: core::task::Poll::Ready
    pub fn into_poll(self) -> Poll<Fut::Output> {
        if let Polling::Ready(out) = self {
            Poll::Ready(out)
        } else {
            Poll::Pending
        }
    }

    // ======================================== Read ======================================== \\

    /// Returns [`true`] is `self` is [`Polling::Ready(_)`].
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.is_ready(), true);
    ///
    /// let polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.is_ready(), false);
    ///
    /// let polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.is_ready(), false);
    /// ```
    ///
    /// [`Polling::Ready(_)`]: Polling::Ready
    pub fn is_ready(&self) -> bool {
        if let Polling::Ready(_) = self {
            true
        } else {
            false
        }
    }

    /// Returns [`true`] is `self` is [`Polling::Pending(_)`].
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.is_pending(), false);
    ///
    /// let polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.is_pending(), true);
    ///
    /// let polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.is_pending(), false);
    /// ```
    ///
    /// [`Polling::Pending(_)`]: Polling::Pending
    pub fn is_pending(&self) -> bool {
        if let Polling::Pending(_) = self {
            true
        } else {
            false
        }
    }

    /// Returns [`true`] is `self` is [`Polling::Done`].
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.is_done(), false);
    ///
    /// let polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.is_done(), false);
    ///
    /// let polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.is_done(), true);
    /// ```
    pub fn is_done(&self) -> bool {
        if let Polling::Done = self {
            true
        } else {
            false
        }
    }

    /// Converts [`Polling::Ready(out)`] into [`Poll::Ready(&out)`], or returns [`Poll::Pending`]
    /// otherwise, without consuming `self`.
    ///
    /// Note that [`Polling::Done`] will also be converted to [`Poll::Pending`].
    ///
    /// ## Example
    ///
    /// ```rust
    /// use core::task::Poll;
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.as_poll(), Poll::Ready(&42));
    ///
    /// let polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.as_poll(), Poll::Pending);
    ///
    /// let polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.as_poll(), Poll::Pending);
    /// ```
    ///
    /// [`Polling::Ready(out)`]: Polling::Ready
    /// [`Poll::Ready(&out)`]: core::task::Poll::Ready
    pub fn as_poll(&self) -> Poll<&Fut::Output> {
        if let Polling::Ready(out) = &self {
            Poll::Ready(out)
        } else {
            Poll::Pending
        }
    }

    // ===================================== Read+Write ===================================== \\

    pub fn replace(&mut self, with: Self) -> Self {
        mem::replace(self, with)
    }

    /// Takes the output or future out of `self`, leaving [`Polling::Done`] in its place.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let mut polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.take().ready(), Some(42));
    /// assert_eq!(polling.ready(), None);
    ///
    /// let mut polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// # future::block_on(async {
    /// assert_eq!(polling.take().pending().unwrap().await, 42);
    /// # });
    /// assert_eq!(polling.pending().is_some(), false);
    ///
    /// let mut polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.take().is_done(), true);
    /// assert_eq!(polling.is_done(), true);
    /// ```
    pub fn take(&mut self) -> Polling<Fut> {
        mem::replace(self, Polling::Done)
    }

    /// If `self` is [`Polling::Ready(out)`], replaces it with [`Polling::Done`] and returns
    /// [`Some(out)`], or returns [`None`] otherwise.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let mut polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.take_ready(), Some(42));
    /// assert_eq!(polling.ready(), None);
    ///
    /// let mut polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.take_ready(), None);
    /// # future::block_on(async {
    /// assert_eq!(polling.pending().unwrap().await, 42);
    /// # });
    ///
    /// let mut polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.take_ready(), None);
    /// assert_eq!(polling.is_done(), true);
    /// ```
    ///
    /// [`Polling::Ready(out)`]: Polling::Ready
    /// [`Some(out)`]: core::option::Option::Some
    pub fn take_ready(&mut self) -> Option<Fut::Output> {
        if self.is_ready() {
            self.take().ready()
        } else {
            None
        }
    }

    /// If `self` is [`Polling::Pending(fut)`], replaces it with [`Polling::Done`] and returns
    /// [`Some(fut)`], or returns [`None`] otherwise.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let mut polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.take_pending().is_some(), false);
    /// assert_eq!(polling.ready(), Some(42));
    ///
    /// let mut polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// # future::block_on(async {
    /// assert_eq!(polling.take_pending().unwrap().await, 42);
    /// # });
    /// assert_eq!(polling.pending().is_some(), false);
    ///
    /// let mut polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.take_pending().is_some(), false);
    /// assert_eq!(polling.is_done(), true);
    /// ```
    ///
    /// [`Polling::Pending(fut)`]: Polling::Pending
    /// [`Some(fut)`]: core::option::Option::Some
    pub fn take_pending(&mut self) -> Option<Fut> {
        if self.is_pending() {
            self.take().pending()
        } else {
            None
        }
    }

    /// If `self` is [`Polling::Ready(out)`], replaces it with [`Polling::Done`] and returns
    /// [`Some(out)`], or returns [`None`] otherwise.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use core::task::Poll;
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let mut polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.take_poll(), Poll::Ready(42));
    /// assert_eq!(polling.ready(), None);
    ///
    /// let mut polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.take_poll(), Poll::Pending);
    /// # future::block_on(async {
    /// assert_eq!(polling.pending().unwrap().await, 42);
    /// # });
    ///
    /// let mut polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.take_poll(), Poll::Pending);
    /// assert_eq!(polling.is_done(), true);
    /// ```
    ///
    /// [`Polling::Ready(out)`]: Polling::Ready
    /// [`Some(out)`]: core::option::Option::Some
    pub fn take_poll(&mut self) -> Poll<Fut::Output> {
        if self.is_ready() {
            self.take().into_poll()
        } else {
            Poll::Pending
        }
    }

    /// If `self` is [`Polling::Pending(fut)`], [polls] `fut` once and returns its [output], or
    /// [`Poll::Pending`] otherwise.
    ///
    /// If `self` is [`Polling::Ready(out)`], replaces it with [`Polling::Done`] and returns
    /// [`Poll::Ready(out)`].
    ///
    /// Panics if `self` is [`Polling::Done`].
    ///
    /// ## Example
    ///
    /// ```rust
    /// use core::task::Poll;
    /// use futures_lite::future::{self, Pending, Ready};
    /// use futures_polling::Polling;
    ///
    /// # future::block_on(async {
    /// #
    /// let mut polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.poll_once().await, Poll::Ready(42));
    /// assert_eq!(polling.is_done(), true);
    ///
    /// let mut polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.poll_once().await, Poll::Ready(42));
    /// assert_eq!(polling.is_done(), true);
    ///
    /// let mut polling = Polling::<Pending<i32>>::Pending(future::pending());
    /// assert_eq!(polling.poll_once().await, Poll::Pending);
    /// assert_eq!(polling.is_done(), false);
    /// #
    /// # });
    /// ```
    ///
    /// [`Polling::Pending(fut)`]: Polling::Pending
    /// [polls]: core::future::Future::poll
    /// [output]: core::future::Future::Output
    /// [`Polling::Ready(out)`]: Polling::Ready
    /// [`Poll::Ready(out)`]: core::task::Poll::Ready
    pub async fn poll_once(&mut self) -> Poll<Fut::Output> {
        if self.is_ready() {
            return self.take().into_poll();
        } else if self.is_done() {
            panic!("output already extracted");
        }

        struct PollOnce<'polling, Fut: Future> {
            polling: &'polling mut Polling<Fut>,
        }

        impl<Fut: Future> Future for PollOnce<'_, Fut> {
            type Output = Poll<Fut::Output>;

            fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
                let this = unsafe { self.get_unchecked_mut() };
                if let Polling::Pending(fut) = this.polling {
                    let poll = unsafe { Pin::new_unchecked(fut) }.poll(ctx);
                    if poll.is_ready() {
                        this.polling.take();
                    }

                    Poll::Ready(poll)
                } else {
                    unreachable!();
                }
            }
        }

        PollOnce { polling: self }.await
    }

    /// If `self` is [`Polling::Pending(fut)`], [polls] `fut` once, replacing `self` with
    /// [`Polling::Ready(out)`] if the future returns [`Poll::Ready(out)`], and returns `self`.
    ///
    /// If `self` is [`Polling::Ready(_)`], returns `self`.
    ///
    /// Panics if `self` is [`Polling::Done`].
    ///
    /// ## Example
    ///
    /// ```rust
    /// use core::task::Poll;
    /// use futures_lite::future::{self, Pending, Ready};
    /// use futures_polling::Polling;
    ///
    /// # future::block_on(async {
    /// #
    /// let mut polling = Polling::<Ready<i32>>::Ready(42);
    /// polling.polling_once().await;
    /// assert_eq!(polling.take_poll(), Poll::Ready(42));
    ///
    /// let mut polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// polling.polling_once().await;
    /// assert_eq!(polling.take_poll(), Poll::Ready(42));
    ///
    /// let mut polling = Polling::<Pending<i32>>::Pending(future::pending());
    /// polling.polling_once().await;
    /// assert_eq!(polling.is_pending(), true);
    /// #
    /// # });
    /// ```
    ///
    /// [`Polling::Pending(fut)`]: Polling::Pending
    /// [polls]: core::future::Future::poll
    /// [`Polling::Ready(out)`]: Polling::Ready
    /// [`Poll::Ready(out)`]: core::task::Poll::Ready
    /// [`Polling::Ready(_)`]: Polling::Ready
    pub async fn polling_once(&mut self) -> &mut Self {
        if self.is_ready() {
            return self;
        } else if self.is_done() {
            panic!("output already extracted");
        }

        struct PollingOnce<'polling, Fut: Future> {
            polling: &'polling mut Polling<Fut>,
        }

        impl<Fut: Future> Future for PollingOnce<'_, Fut> {
            type Output = ();

            fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
                let this = unsafe { self.get_unchecked_mut() };
                if let Polling::Pending(fut) = this.polling {
                    if let Poll::Ready(out) = unsafe { Pin::new_unchecked(fut) }.poll(ctx) {
                        this.polling.replace(Polling::Ready(out));
                    }

                    Poll::Ready(())
                } else {
                    unreachable!();
                }
            }
        }

        PollingOnce { polling: self }.await;
        self
    }
}

// ========================================= impl Future ======================================== \\

impl<Fut: Future> Future for Polling<Fut> {
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        match this {
            Polling::Ready(_) => this.take().into_poll(),
            Polling::Pending(fut) => {
                let poll = unsafe { Pin::new_unchecked(fut) }.poll(ctx);
                if poll.is_ready() {
                    this.take();
                }

                poll
            },
            Polling::Done => panic!("output already extracted"),
        }
    }
}

// ========================================== impl From ========================================= \\

impl<Fut: Future> From<Fut> for Polling<Fut> {
    fn from(fut: Fut) -> Self {
        Polling::Pending(fut)
    }
}
