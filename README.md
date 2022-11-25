Popula community
==================

An all-in-one template for general purpose communities on Near.


Features
===========
### User Contents
Users can add thier post, multi level comment and like action to community. These raw data stores in transactions, and smart contract only stores their hashes. But here we only stores the previous 28 bits of a hash through an optimized bit tree.

### Role Management
Includes variety of permissions that a role might use. And contains mod level for different roles even a role that only manage specific one other role for purpose. A global role is used then all roles can be configured by a single transaction.

### Deposit System
Since it's difficult to prove someone's assets on chain, we provide deposit system to simplified the procedure. It is not only for Near token but all NEP-141 tokens and NtFt which is integrated in drip protocol.

### Points Recorder
We defined a series of point map for every action as well as drips, so anyone can know who is more active in a community. And those drips can be collected by a verified NtFt contract then users can prove it to a deposit system not only for this community. 


Exploring The Code
==================

1. The main smart contract code lives in `src/lib.rs`. You can compile it with
   the `./compile` script.
2. Tests: You can run smart contract tests with the `./test` script. This runs
   standard Rust tests using [cargo] with a `--nocapture` flag so that you
   can see any debug info you print to the console.


  [smart contract]: https://docs.near.org/docs/develop/contracts/overview
  [Rust]: https://www.rust-lang.org/
  [create-near-app]: https://github.com/near/create-near-app
  [correct target]: https://github.com/near/near-sdk-rs#pre-requisites
  [cargo]: https://doc.rust-lang.org/book/ch01-03-hello-cargo.html
