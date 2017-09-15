NS Router Crate
===============

[Github](https://github.com/tailhook/ns-router) |
[Documentaion](http://docs.rs/ns-router) |
[Crate](https://crates.io/crates/ns-router)

A configurable name resolver for [abstract-ns][1]. It does not have any actual
name resolution implementations, but it allows:

* Resolve different names by different means (say resolve ``*.consul`` in
  consul, but other TLDs using normal DNS)
* Make a [stream][2] of addresses (i.e. resolve and update) from a list of names
* Make a stream of addresses from stream of name lists so you can update config
  on the fly
* Reconfigure router on the fly and get all streams updated


[1]: https://crates.io/crates/abstract-ns
[2]: https://docs.rs/futures/0.1.16/futures/stream/trait.Stream.html


License
=======

Licensed under either of

* Apache License, Version 2.0,
  (./LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (./LICENSE-MIT or http://opensource.org/licenses/MIT)
  at your option.

Contribution
------------

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

