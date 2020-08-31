/**************************************************************************************************
 *                                                                                                *
 * This Source Code Form is subject to the terms of the Mozilla Public                            *
 * License, v. 2.0. If a copy of the MPL was not distributed with this                            *
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.                                       *
 *                                                                                                *
 **************************************************************************************************/

// =========================================== Imports ========================================== \\

pub mod prelude {
    pub use crate::Polling;
    pub use crate::FuturePollingExt as _;
}

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

pub trait FuturePollingExt: Future {
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

    pub fn ready(self) -> Option<Fut::Output> {
        if let Polling::Ready(out) = self {
            Some(out)
        } else {
            None
        }
    }

    pub fn pending(self) -> Option<Fut> {
        if let Polling::Pending(fut) = self {
            Some(fut)
        } else {
            None
        }
    }

    pub fn into_poll(self) -> Poll<Fut::Output> {
        if let Polling::Ready(out) = self {
            Poll::Ready(out)
        } else {
            Poll::Pending
        }
    }

    // ======================================== Read ======================================== \\

    pub fn is_ready(&self) -> bool {
        if let Polling::Ready(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_pending(&self) -> bool {
        if let Polling::Pending(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_done(&self) -> bool {
        if let Polling::Done = self {
            true
        } else {
            false
        }
    }

    pub fn as_poll(&self) -> Poll<&Fut::Output> {
        if let Polling::Ready(out) = &self {
            Poll::Ready(out)
        } else {
            Poll::Pending
        }
    }

    // ===================================== Read+Write ===================================== \\

    pub fn take(&mut self) -> Polling<Fut> {
        mem::replace(self, Polling::Done)
    }

    pub fn take_ready(&mut self) -> Option<Fut::Output> {
        self.take().ready()
    }

    pub fn take_pending(&mut self) -> Option<Fut> {
        self.take().pending()
    }

    pub fn take_poll(&mut self) -> Poll<Fut::Output> {
        self.take().into_poll()
    }

    pub async fn poll_once(&mut self) -> Poll<Fut::Output> {
        struct PollOnce<'polling, Fut: Future> {
            polling: &'polling mut Polling<Fut>,
        }

        impl<Fut: Future> Future for PollOnce<'_, Fut> {
            type Output = Poll<Fut::Output>;

            fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
                let this = unsafe { self.get_unchecked_mut() };
                Poll::Ready(unsafe { Pin::new_unchecked(&mut *this.polling) }.poll(ctx))
            }
        }

        PollOnce { polling: self }.await
    }
}

// ========================================= impl Future ======================================== \\

impl<Fut: Future> Future for Polling<Fut> {
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        match this {
            Polling::Ready(_) => this.take_poll(),
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
