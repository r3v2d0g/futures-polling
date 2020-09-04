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
/// An enum similar to [`Poll`], but containing a [future] in its `Pending` variant.
///
/// ## Example
///
/// ```rust
/// use futures_lite::future;
/// use futures_polling::{FuturePollingExt, Polling};
///
/// # future::block_on(async {
/// #
/// let mut polling = async {
///     future::yield_now().await;
///     42
/// }.polling();
///
/// assert_eq!(polling.is_pending(), true);
///
/// // Poll just once.
/// polling.polling_once().await;
/// assert_eq!(polling.is_pending(), true);
///
/// // Poll until the inner future is ready.
/// assert_eq!(polling.await, 42);
/// #
/// # });
/// ```
///
/// [future]: core::future::Future
pub enum Polling<Fut: Future> {
    /// Contains the [future's output] once it has returned it (like [`Poll::Ready`]).
    ///
    /// ## Example
    ///
    /// ```rust
    /// # use futures_lite::future;
    /// use futures_polling::{FuturePollingExt, Polling};
    ///
    /// # future::block_on(async {
    /// #
    /// let mut polling = async { 42i32 }.polling();
    /// polling.polling_once().await;
    ///
    /// if let Polling::Ready(out) = polling {
    ///     assert_eq!(out, 42);
    /// } else {
    ///     unreachable!();
    /// }
    ///
    /// // or
    ///
    /// assert_eq!(polling.into_ready(), Some(42));
    /// #
    /// # });
    /// ```
    ///
    /// [future's output]: core::future::Future::Output
    Ready(Fut::Output),
    /// Contains the pending [future].
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future;
    /// use futures_polling::{FuturePollingExt, Polling};
    ///
    /// # future::block_on(async {
    /// #
    /// let mut polling = async {
    ///     future::yield_now().await;
    ///     42i32
    /// }.polling();
    ///
    /// if let Polling::Pending(_) = polling {
    ///     // -> future::yield_now().await;
    ///     polling.polling_once().await;
    /// } else {
    ///     unreachable!();
    /// }
    ///
    /// if let Polling::Pending(_) = polling {
    ///     // 42
    ///     polling.polling_once().await;
    /// } else {
    ///     unreachable!();
    /// }
    ///
    /// assert_eq!(polling.into_ready(), Some(42));
    /// #
    /// # });
    /// ```
    ///
    /// [future]: core::future::Future
    Pending(Fut),
    /// The [future] has already returned an [output], but it has already been [extracted] out of
    /// `Polling`.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # use futures_lite::future;
    /// use futures_polling::{FuturePollingExt, Polling};
    ///
    /// # future::block_on(async {
    /// #
    /// let mut polling = async { 42i32 }.polling();
    /// polling.polling_once().await;
    ///
    /// assert_eq!(polling.take_ready(), Some(42));
    /// assert_eq!(polling.is_done(), true);
    /// #
    /// # });
    /// ```
    ///
    /// [future]: core::future::Future
    /// [extracted]: Polling::take_ready()
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

    /// |         `self`        |    return   |
    /// |-----------------------|-------------|
    /// | `Polling::Ready(out)` | `Some(out)` |
    /// | `Polling::Pending(_)` | `None`      |
    /// | `Polling::Done`       | `None`      |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.into_ready(), Some(42));
    ///
    /// let polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.into_ready(), None);
    ///
    /// let polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.into_ready(), None);
    /// ```
    ///
    /// [`Polling::Ready(out)`]: Polling::Ready
    /// [`Some(out)`]: core::option::Option::Some
    /// [`as_ready()`]: Polling::as_ready()
    /// [`as_ready_mut()`]: Polling::as_ready_mut()
    pub fn into_ready(self) -> Option<Fut::Output> {
        if let Polling::Ready(out) = self {
            Some(out)
        } else {
            None
        }
    }

    /// |          `self`         |    return   |
    /// |-------------------------|-------------|
    /// | `Polling::Ready(_)`     | `None`      |
    /// | `Polling::Pending(fut)` | `Some(fut)` |
    /// | `Polling::Done`         | `None`      |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.into_pending().is_some(), false);
    ///
    /// let polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// # future::block_on(async {
    /// assert_eq!(polling.into_pending().unwrap().await, 42);
    /// # });
    ///
    /// let polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.into_pending().is_some(), false);
    /// ```
    ///
    /// [`Polling::Pending(fut)`]: Polling::Pending
    /// [`Some(fut)`]: core::option::Option::Some
    /// [`as_pending()`]: Polling::as_pending()
    /// [`as_pending_mut()`]: Polling::as_pending_mut()
    pub fn into_pending(self) -> Option<Fut> {
        if let Polling::Pending(fut) = self {
            Some(fut)
        } else {
            None
        }
    }

    /// |         `self`        |       return       |
    /// |-----------------------|--------------------|
    /// | `Polling::Ready(out)` | `Poll::Ready(out)` |
    /// | `Polling::Pending(_)` | `Poll::Pending`    |
    /// | `Polling::Done`       | `Poll::Pending`    |
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
    /// [`as_poll()`]: Polling::as_poll()
    /// [`as_poll_mut()`]: Polling::as_poll_mut()
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

    /// |        `&self`        |    return    |
    /// |-----------------------|--------------|
    /// | `Polling::Ready(out)` | `Some(&out)` |
    /// | `Polling::Pending(_)` | `None`       |
    /// | `Polling::Done`       | `None`       |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.as_ready(), Some(&42));
    ///
    /// let polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.as_ready(), None);
    ///
    /// let polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.as_ready(), None);
    /// ```
    ///
    /// [`Polling::Ready(out)`]: Polling::Ready
    /// [`Some(&out)`]: core::option::Option::Some
    /// [`as_ready_mut()`]: Polling::as_ready_mut()
    /// [`into_ready()`]: Polling::into_ready()
    pub fn as_ready(&self) -> Option<&Fut::Output> {
        if let Polling::Ready(out) = self {
            Some(out)
        } else {
            None
        }
    }

    /// |         `&self`         |    return    |
    /// |-------------------------|--------------|
    /// | `Polling::Ready(_)`     | `None`       |
    /// | `Polling::Pending(fut)` | `Some(&fut)` |
    /// | `Polling::Done`         | `None`       |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.as_pending().is_some(), false);
    ///
    /// let polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.as_pending().is_some(), true);
    ///
    /// let polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.as_pending().is_some(), false);
    /// ```
    ///
    /// [`Polling::Pending(fut)`]: Polling::Pending
    /// [`Some(&fut)`]: core::option::Option::Some
    /// [`as_pending_mut()`]: Polling::as_pending_mut()
    /// [`into_pending()`]: Polling::into_pending()
    pub fn as_pending(&self) -> Option<&Fut> {
        if let Polling::Pending(fut) = self {
            Some(fut)
        } else {
            None
        }
    }

    /// |         `&self`       |        return       |
    /// |-----------------------|---------------------|
    /// | `Polling::Ready(out)` | `Poll::Ready(&out)` |
    /// | `Polling::Pending(_)` | `Poll::Pending`     |
    /// | `Polling::Done`       | `Poll::Pending`     |
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
    /// [`as_poll_mut()`]: Polling::as_poll_mut()
    /// [`into_poll()`]: Polling::into_poll()
    pub fn as_poll(&self) -> Poll<&Fut::Output> {
        if let Polling::Ready(out) = self {
            Poll::Ready(out)
        } else {
            Poll::Pending
        }
    }

    /// |      `&mut self`      |      return      |
    /// |-----------------------|------------------|
    /// | `Polling::Ready(out)` | `Some(&mut out)` |
    /// | `Polling::Pending(_)` | `None`           |
    /// | `Polling::Done`       | `None`           |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let mut polling = Polling::<Ready<i32>>::Ready(42);
    /// if let Some(out) = polling.as_ready_mut() {
    ///     assert_eq!(*out, 42);
    ///     *out = 0;
    /// } else {
    ///     unreachable!();
    /// }
    /// assert_eq!(polling.as_ready(), Some(&0));
    ///
    /// let mut polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.as_ready_mut(), None);
    ///
    /// let mut polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.as_ready_mut(), None);
    /// ```
    ///
    /// [`Polling::Ready(out)`]: Polling::Ready
    /// [`Some(&mut out)`]: core::option::Option::Some
    /// [`as_ready()`]: Polling::as_ready()
    /// [`into_ready()`]: Polling::into_ready()
    pub fn as_ready_mut(&mut self) -> Option<&mut Fut::Output> {
        if let Polling::Ready(out) = self {
            Some(out)
        } else {
            None
        }
    }

    /// |       `&mut self`       |      return      |
    /// |-------------------------|------------------|
    /// | `Polling::Ready(_)`     | `None`           |
    /// | `Polling::Pending(fut)` | `Some(&mut fut)` |
    /// | `Polling::Done`         | `None`           |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let mut polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.as_pending_mut().is_some(), false);
    ///
    /// let mut polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// # future::block_on(async {
    /// assert_eq!(polling.as_pending_mut().unwrap().await, 42);
    /// # });
    ///
    /// let mut polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.as_pending_mut().is_some(), false);
    /// ```
    ///
    /// [`Polling::Pending(fut)`]: Polling::Pending
    /// [`Some(&mut fut)`]: core::option::Option::Some
    /// [`as_pending()`]: Polling::as_pending()
    /// [`into_pending()`]: Polling::into_pending()
    pub fn as_pending_mut(&mut self) -> Option<&mut Fut> {
        if let Polling::Pending(fut) = self {
            Some(fut)
        } else {
            None
        }
    }

    /// |       `&mut self`     |          return         |
    /// |-----------------------|-------------------------|
    /// | `Polling::Ready(out)` | `Poll::Ready(&mut out)` |
    /// | `Polling::Pending(_)` | `Poll::Pending`         |
    /// | `Polling::Done`       | `Poll::Pending`         |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use core::task::Poll;
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let mut polling = Polling::<Ready<i32>>::Ready(42);
    /// if let Poll::Ready(out) = polling.as_poll_mut() {
    ///     *out = 0;
    /// } else {
    ///     unreachable!();
    /// }
    /// assert_eq!(polling.as_poll(), Poll::Ready(&0));
    ///
    /// let mut polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.as_poll_mut(), Poll::Pending);
    ///
    /// let mut polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.as_poll_mut(), Poll::Pending);
    /// ```
    ///
    /// [`Polling::Ready(out)`]: Polling::Ready
    /// [`Poll::Ready(&out)`]: core::task::Poll::Ready
    /// [`as_poll_mut()`]: Polling::as_poll_mut()
    /// [`into_poll()`]: Polling::into_poll()
    pub fn as_poll_mut(&mut self) -> Poll<&mut Fut::Output> {
        if let Polling::Ready(out) = self {
            Poll::Ready(out)
        } else {
            Poll::Pending
        }
    }

    // ===================================== Read+Write ===================================== \\

    /// Moves `with` into `&mut self` and returns the previous `self`.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let mut p1 = Polling::<Ready<i32>>::Ready(42);
    /// let p2 = Polling::<Ready<i32>>::Ready(24);
    ///
    /// let p3 = p1.replace(p2);
    ///
    /// assert_eq!(p1.as_ready(), Some(&24));
    /// assert_eq!(p3.as_ready(), Some(&42));
    /// ```
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
    /// assert_eq!(polling.take().into_ready(), Some(42));
    /// assert_eq!(polling.into_ready(), None);
    ///
    /// let mut polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// # future::block_on(async {
    /// assert_eq!(polling.take().into_pending().unwrap().await, 42);
    /// # });
    /// assert_eq!(polling.into_pending().is_some(), false);
    ///
    /// let mut polling = Polling::<Ready<i32>>::Done;
    /// assert_eq!(polling.take().is_done(), true);
    /// assert_eq!(polling.is_done(), true);
    /// ```
    pub fn take(&mut self) -> Polling<Fut> {
        mem::replace(self, Polling::Done)
    }

    /// |       `&mut self`       |     new `self` value    |    return   |
    /// |-------------------------|-------------------------|-------------|
    /// | `Polling::Ready(out)`   | `Polling::Done`         | `Some(out)` |
    /// | `Polling::Pending(fut)` | `Polling::Pending(fut)` | `None`      |
    /// | `Polling::Done`         | `Polling::Done`         | `None`      |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let mut polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.take_ready(), Some(42));
    /// assert_eq!(polling.into_ready(), None);
    ///
    /// let mut polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.take_ready(), None);
    /// # future::block_on(async {
    /// assert_eq!(polling.into_pending().unwrap().await, 42);
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
            self.take().into_ready()
        } else {
            None
        }
    }

    /// |       `&mut self`       |    new `self` value   |    return   |
    /// |-------------------------|-----------------------|-------------|
    /// | `Polling::Ready(out)`   | `Polling::Ready(out)` | `None`      |
    /// | `Polling::Pending(fut)` | `Polling::Done`       | `Some(fut)` |
    /// | `Polling::Done`         | `Polling::Done`       | `None`      |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let mut polling = Polling::<Ready<i32>>::Ready(42);
    /// assert_eq!(polling.take_pending().is_some(), false);
    /// assert_eq!(polling.into_ready(), Some(42));
    ///
    /// let mut polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// # future::block_on(async {
    /// assert_eq!(polling.take_pending().unwrap().await, 42);
    /// # });
    /// assert_eq!(polling.into_pending().is_some(), false);
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
            self.take().into_pending()
        } else {
            None
        }
    }

    /// |       `&mut self`       |     new `self` value    |       return       |
    /// |-------------------------|-------------------------|--------------------|
    /// | `Polling::Ready(out)`   | `Polling::Done`         | `Poll::Ready(out)` |
    /// | `Polling::Pending(fut)` | `Polling::Pending(fut)` | `Poll::Pending`    |
    /// | `Polling::Done`         | `Polling::Done`         | `Poll::Pending`    |
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
    /// assert_eq!(polling.into_ready(), None);
    ///
    /// let mut polling = Polling::<Ready<i32>>::Pending(future::ready(42));
    /// assert_eq!(polling.take_poll(), Poll::Pending);
    /// # future::block_on(async {
    /// assert_eq!(polling.into_pending().unwrap().await, 42);
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

    /// |       `&mut self`       |    `fut.poll(_)`   |     new `self` value    |       return       |
    /// |-------------------------|--------------------|-------------------------|--------------------|
    /// | `Polling::Ready(out)`   |          x         | `Polling::Done`         | `Poll::Ready(out)` |
    /// | `Polling::Pending(fut)` | `Poll::Ready(out)` | `Polling::Done`         | `Poll::Ready(out)` |
    /// | `Polling::Pending(fut)` | `Poll::Pending`    | `Polling::Pending(fut)` | `Poll::Pending`    |
    /// | `Polling::Done`         |          x         |         panic!()        |      panic!()      |
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
        self.polling_once().await;
        self.take_poll()
    }

    /// |       `&mut self`       |    `fut.poll(_)`   |     new `self` value    |
    /// |-------------------------|--------------------|-------------------------|
    /// | `Polling::Ready(out)`   |          x         | `Polling::Ready(out)`   |
    /// | `Polling::Pending(fut)` | `Poll::Ready(out)` | `Polling::Ready(out)`   |
    /// | `Polling::Pending(fut)` | `Poll::Pending`    | `Polling::Pending(fut)` |
    /// | `Polling::Done`         |          x         |         panic!()        |
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

impl<T, E, Fut: Future<Output = Result<T, E>>> Polling<Fut> {
    // ===================================== Destructors ==================================== \\

    /// |           `self`           |     return     |
    /// |----------------------------|----------------|
    /// | `Polling::Ready(Ok(ok))`   | `Ok(Some(ok))` |
    /// | `Polling::Ready(Err(err))` | `Err(err)`     |
    /// | `Polling::Pending(_)`      | `Ok(None)`     |
    /// | `Polling::Done`            | `Ok(None)`     |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Ready(Ok(42));
    /// assert_eq!(polling.try_into_ready(), Ok(Some(42)));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Ready(Err(42));
    /// assert_eq!(polling.try_into_ready(), Err(42));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Ok(42)));
    /// assert_eq!(polling.try_into_ready(), Ok(None));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Err(42)));
    /// assert_eq!(polling.try_into_ready(), Ok(None));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Done;
    /// assert_eq!(polling.try_into_ready(), Ok(None));
    /// ```
    pub fn try_into_ready(self) -> Result<Option<T>, E> {
        self.into_ready().transpose()
    }

    /// |           `self`           |         return        |
    /// |----------------------------|-----------------------|
    /// | `Polling::Ready(Ok(ok))`   | `Ok(Poll::Ready(ok))` |
    /// | `Polling::Ready(Err(err))` | `Err(err)`            |
    /// | `Polling::Pending(_)`      | `Ok(Poll::Pending)`   |
    /// | `Polling::Done`            | `Ok(Poll::Pending)`   |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use core::task::Poll;
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Ready(Ok(42));
    /// assert_eq!(polling.try_into_poll(), Ok(Poll::Ready(42)));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Ready(Err(42));
    /// assert_eq!(polling.try_into_poll(), Err(42));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Ok(42)));
    /// assert_eq!(polling.try_into_poll(), Ok(Poll::Pending));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Err(42)));
    /// assert_eq!(polling.try_into_poll(), Ok(Poll::Pending));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Done;
    /// assert_eq!(polling.try_into_poll(), Ok(Poll::Pending));
    /// ```
    pub fn try_into_poll(self) -> Result<Poll<T>, E> {
        match self.into_poll() {
            Poll::Ready(Ok(ok)) => Ok(Poll::Ready(ok)),
            Poll::Ready(Err(err)) => Err(err),
            Poll::Pending => Ok(Poll::Pending),
        }
    }

    // ======================================== Read ======================================== \\

    /// |          `&self`           |      return     |
    /// |----------------------------|-----------------|
    /// | `Polling::Ready(Ok(ok))`   | `Ok(Some(&ok))` |
    /// | `Polling::Ready(Err(Err))` | `Err(&err)`     |
    /// | `Polling::Pending(_)`      | `Ok(None)`      |
    /// | `Polling::Done`            | `Ok(None)`      |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Ready(Ok(42));
    /// assert_eq!(polling.try_as_ready(), Ok(Some(&42)));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Ready(Err(42));
    /// assert_eq!(polling.try_as_ready(), Err(&42));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Ok(42)));
    /// assert_eq!(polling.try_as_ready(), Ok(None));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Err(42)));
    /// assert_eq!(polling.try_as_ready(), Ok(None));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Done;
    /// assert_eq!(polling.try_as_ready(), Ok(None));
    /// ```
    pub fn try_as_ready(&self) -> Result<Option<&T>, &E> {
        self.as_ready().map(Result::as_ref).transpose()
    }

    /// |          `&self`           |         return         |
    /// |----------------------------|------------------------|
    /// | `Polling::Ready(Ok(ok))`   | `Ok(Poll::Ready(&ok))` |
    /// | `Polling::Ready(Err(Err))` | `Err(&mut err)`        |
    /// | `Polling::Pending(_)`      | `Ok(Poll::Pending)`    |
    /// | `Polling::Done`            | `Ok(Poll::Pending)`    |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use core::task::Poll;
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Ready(Ok(42));
    /// assert_eq!(polling.try_as_poll(), Ok(Poll::Ready(&42)));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Ready(Err(42));
    /// assert_eq!(polling.try_as_poll(), Err(&42));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Ok(42)));
    /// assert_eq!(polling.try_as_poll(), Ok(Poll::Pending));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Err(42)));
    /// assert_eq!(polling.try_as_poll(), Ok(Poll::Pending));
    ///
    /// let polling = Polling::<Ready<Result<i32, i32>>>::Done;
    /// assert_eq!(polling.try_as_poll(), Ok(Poll::Pending));
    /// ```
    pub fn try_as_poll(&self) -> Result<Poll<&T>, &E> {
        match self.as_poll() {
            Poll::Ready(Ok(ok)) => Ok(Poll::Ready(ok)),
            Poll::Ready(Err(err)) => Err(err),
            Poll::Pending => Ok(Poll::Pending),
        }
    }

    /// |        `&mut self`         |        return       |
    /// |----------------------------|---------------------|
    /// | `Polling::Ready(Ok(ok))`   | `Ok(Some(&mut ok))` |
    /// | `Polling::Ready(Err(Err))` | `Err(&mut err)`     |
    /// | `Polling::Pending(_)`      | `Ok(None)`          |
    /// | `Polling::Done`            | `Ok(None)`          |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Ready(Ok(42));
    /// if let Ok(Some(ok)) = polling.try_as_ready_mut() {
    ///     assert_eq!(*ok, 42);
    ///     *ok = 0;
    /// } else {
    ///     unreachable!();
    /// }
    /// assert_eq!(polling.try_as_ready(), Ok(Some(&0)));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Ready(Err(42));
    /// if let Err(err) = polling.try_as_ready_mut() {
    ///     assert_eq!(*err, 42);
    ///     *err = 0;
    /// } else {
    ///     unreachable!();
    /// }
    /// assert_eq!(polling.try_as_ready(), Err(&0));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Ok(42)));
    /// assert_eq!(polling.try_as_ready_mut(), Ok(None));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Err(42)));
    /// assert_eq!(polling.try_as_ready_mut(), Ok(None));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Done;
    /// assert_eq!(polling.try_as_ready_mut(), Ok(None));
    /// ```
    pub fn try_as_ready_mut(&mut self) -> Result<Option<&mut T>, &mut E> {
        self.as_ready_mut().map(Result::as_mut).transpose()
    }

    /// |        `&mut self`         |           return           |
    /// |----------------------------|----------------------------|
    /// | `Polling::Ready(Ok(ok))`   | `Ok(Poll::Ready(&mut ok))` |
    /// | `Polling::Ready(Err(Err))` | `Err(&mut err)`            |
    /// | `Polling::Pending(_)`      | `Ok(Poll::Pending)`        |
    /// | `Polling::Done`            | `Ok(Poll::Pending)`        |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use core::task::Poll;
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Ready(Ok(42));
    /// if let Ok(Poll::Ready(ok)) = polling.try_as_poll_mut() {
    ///     assert_eq!(*ok, 42);
    ///     *ok = 0;
    /// } else {
    ///     unreachable!();
    /// }
    /// assert_eq!(polling.try_as_poll(), Ok(Poll::Ready(&0)));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Ready(Err(42));
    /// if let Err(err) = polling.try_as_poll_mut() {
    ///     assert_eq!(*err, 42);
    ///     *err = 0;
    /// } else {
    ///     unreachable!();
    /// }
    /// assert_eq!(polling.try_as_poll(), Err(&0));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Ok(42)));
    /// assert_eq!(polling.try_as_poll_mut(), Ok(Poll::Pending));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Err(42)));
    /// assert_eq!(polling.try_as_poll_mut(), Ok(Poll::Pending));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Done;
    /// assert_eq!(polling.try_as_poll_mut(), Ok(Poll::Pending));
    /// ```
    pub fn try_as_poll_mut(&mut self) -> Result<Poll<&mut T>, &mut E> {
        match self.as_poll_mut() {
            Poll::Ready(Ok(ok)) => Ok(Poll::Ready(ok)),
            Poll::Ready(Err(err)) => Err(err),
            Poll::Pending => Ok(Poll::Pending),
        }
    }

    // ===================================== Read+Write ===================================== \\

    /// |         `&mut self`        |     new `self` value    |     return     |
    /// |----------------------------|-------------------------|----------------|
    /// | `Polling::Ready(Ok(ok))`   | `Polling::Done`         | `Ok(Some(ok))` |
    /// | `Polling::Ready(Err(err))` | `Polling::Done`         | `Err(err)`     |
    /// | `Polling::Pending(fut)`    | `Polling::Pending(fut)` | `Ok(None)`     |
    /// | `Polling::Done`            | `Polling::Done`         | `Ok(None)`     |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Ready(Ok(42));
    /// assert_eq!(polling.try_take_ready(), Ok(Some(42)));
    /// assert_eq!(polling.try_into_ready(), Ok(None));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Ready(Err(42));
    /// assert_eq!(polling.try_take_ready(), Err(42));
    /// assert_eq!(polling.try_into_ready(), Ok(None));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Ok(42)));
    /// assert_eq!(polling.try_take_ready(), Ok(None));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Err(42)));
    /// assert_eq!(polling.try_take_ready(), Ok(None));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Done;
    /// assert_eq!(polling.try_take_ready(), Ok(None));
    /// ```
    pub fn try_take_ready(&mut self) -> Result<Option<T>, E> {
        self.take_ready().transpose()
    }

    /// |         `&mut self`        |     new `self` value    |         return        |
    /// |----------------------------|-------------------------|-----------------------|
    /// | `Polling::Ready(Ok(ok))`   | `Polling::Done`         | `Ok(Poll::Ready(ok))` |
    /// | `Polling::Ready(Err(err))` | `Polling::Done`         | `Err(err)`            |
    /// | `Polling::Pending(fut)`    | `Polling::Pending(fut)` | `Ok(Poll::Pending)`   |
    /// | `Polling::Done`            | `Polling::Done`         | `Ok(Poll::Pending)`   |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use core::task::Poll;
    /// use futures_lite::future::{self, Ready};
    /// use futures_polling::Polling;
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Ready(Ok(42));
    /// assert_eq!(polling.try_take_poll(), Ok(Poll::Ready(42)));
    /// assert_eq!(polling.try_into_ready(), Ok(None));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Ready(Err(42));
    /// assert_eq!(polling.try_take_poll(), Err(42));
    /// assert_eq!(polling.try_into_ready(), Ok(None));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Ok(42)));
    /// assert_eq!(polling.try_take_poll(), Ok(Poll::Pending));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Err(42)));
    /// assert_eq!(polling.try_take_poll(), Ok(Poll::Pending));
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Done;
    /// assert_eq!(polling.try_take_poll(), Ok(Poll::Pending));
    /// ```
    pub fn try_take_poll(&mut self) -> Result<Poll<T>, E> {
        match self.take_poll() {
            Poll::Ready(Ok(ok)) => Ok(Poll::Ready(ok)),
            Poll::Ready(Err(err)) => Err(err),
            Poll::Pending => Ok(Poll::Pending),
        }
    }

    /// |         `&mut self`        |     `fut.poll(ctx)`     |     new `self` value    |         return        |
    /// |----------------------------|-------------------------|-------------------------|-----------------------|
    /// | `Polling::Ready(Ok(ok))`   |            x            | `Polling::Done`         | `Ok(Poll:Ready(ok))`  |
    /// | `Polling::Ready(Err(err))` |            x            | `Polling::Done`         | `Err(err)`            |
    /// | `Polling::Pending(fut)`    | `Poll::Ready(Ok(ok))`   | `Polling::Done`         | `Ok(Poll::Ready(ok))` |
    /// | `Polling::Pending(fut)`    | `Poll::Ready(Err(err))` | `Polling::Done`         | `Err(err)`            |
    /// | `Polling::Pending(fut)`    | `Poll::Pending`         | `Polling::Pending(fut)` | `Ok(Poll::Pending)`   |
    /// | `Polling::Done`            |            x            |         panic!()        |        panic!()       |
    ///
    /// ## Example
    ///
    /// ```rust
    /// use core::pin::Pin;
    /// use core::task::Poll;
    /// use futures_lite::future::{self, Pending, Ready};
    /// use futures_polling::Polling;
    ///
    /// # future::block_on(future::poll_fn(|ctx| {
    /// #
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Ready(Ok(42));
    /// assert_eq!(Pin::new(&mut polling).try_poll(ctx), Ok(Poll::Ready(42)));
    /// assert_eq!(polling.is_done(), true);
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Ready(Err(42));
    /// assert_eq!(Pin::new(&mut polling).try_poll(ctx), Err(42));
    /// assert_eq!(polling.is_done(), true);
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Ok(42)));
    /// assert_eq!(Pin::new(&mut polling).try_poll(ctx), Ok(Poll::Ready(42)));
    /// assert_eq!(polling.is_done(), true);
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Err(42)));
    /// assert_eq!(Pin::new(&mut polling).try_poll(ctx), Err(42));
    /// assert_eq!(polling.is_done(), true);
    ///
    /// let mut polling = Polling::<Pending<Result<i32, i32>>>::Pending(future::pending());
    /// assert_eq!(Pin::new(&mut polling).try_poll(ctx), Ok(Poll::Pending));
    /// assert_eq!(polling.is_done(), false);
    /// #
    /// # Poll::Ready(()) }));
    /// ```
    pub fn try_poll(self: Pin<&mut Self>, ctx: &mut Context) -> Result<Poll<T>, E> {
        match self.poll(ctx) {
            Poll::Ready(Ok(ok)) => Ok(Poll::Ready(ok)),
            Poll::Ready(Err(err)) => Err(err),
            Poll::Pending => Ok(Poll::Pending),
        }
    }

    /// |         `&mut self`        |      `fut.poll(_)`      |     new `self` value    |         return        |
    /// |----------------------------|-------------------------|-------------------------|-----------------------|
    /// | `Polling::Ready(Ok(ok))`   |            x            | `Polling::Done`         | `Ok(Poll:Ready(ok))`  |
    /// | `Polling::Ready(Err(err))` |            x            | `Polling::Done`         | `Err(err)`            |
    /// | `Polling::Pending(fut)`    | `Poll::Ready(Ok(ok))`   | `Polling::Done`         | `Ok(Poll::Ready(ok))` |
    /// | `Polling::Pending(fut)`    | `Poll::Ready(Err(err))` | `Polling::Done`         | `Err(err)`            |
    /// | `Polling::Pending(fut)`    | `Poll::Pending`         | `Polling::Pending(fut)` | `Ok(Poll::Pending)`   |
    /// | `Polling::Done`            |            x            |         panic!()        |        panic!()       |
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
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Ready(Ok(42));
    /// assert_eq!(polling.try_poll_once().await, Ok(Poll::Ready(42)));
    /// assert_eq!(polling.is_done(), true);
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Ready(Err(42));
    /// assert_eq!(polling.try_poll_once().await, Err(42));
    /// assert_eq!(polling.is_done(), true);
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Ok(42)));
    /// assert_eq!(polling.try_poll_once().await, Ok(Poll::Ready(42)));
    /// assert_eq!(polling.is_done(), true);
    ///
    /// let mut polling = Polling::<Ready<Result<i32, i32>>>::Pending(future::ready(Err(42)));
    /// assert_eq!(polling.try_poll_once().await, Err(42));
    /// assert_eq!(polling.is_done(), true);
    ///
    /// let mut polling = Polling::<Pending<Result<i32, i32>>>::Pending(future::pending());
    /// assert_eq!(polling.try_poll_once().await, Ok(Poll::Pending));
    /// assert_eq!(polling.is_done(), false);
    /// #
    /// # });
    /// ```
    pub async fn try_poll_once(&mut self) -> Result<Poll<T>, E> {
        self.polling_once().await;
        self.try_take_poll()
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
