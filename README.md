# An enum similar to Poll, but containing a future in its Pending variant

[![img](https://img.shields.io/crates/l/futures-polling.svg)](https://github.com/r3v2d0g/futures-polling/blob/main/LICENSE.txt) [![img](https://img.shields.io/crates/v/futures-polling.svg)](https://crates.io/crates/futures-polling) [![img](https://docs.rs/futures-polling/badge.svg)](https://docs.rs/futures-polling)


## Example

```rust
use futures_lite::future;
use futures_polling::{FuturePollingExt, Polling};

let mut polling = async {
    future::yield_now().await;
    42
}.polling();

assert_eq!(polling.is_pending(), true);

// Poll just once.
polling.polling_once().await;
assert_eq!(polling.is_pending(), true);

// Poll until the inner future is ready.
assert_eq!(polling.await, 42);
```


## License

> This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at <http://mozilla.org/MPL/2.0/>.
