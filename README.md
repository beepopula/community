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

## Terminology

* `owner_id`: The owner of this contract.
* `args`: Some customized arguments.
* `accounts`: Includes Points, deposit, and any possible arguments.
* `content_tree`: Tree map for large scale contents, for now it stores posts, comments.  
* `relationship_tree`: Tree map for like, share and follow actions.
* `reports`: To record who's reporting user contents.
* `drip`: Functions to give users points.
* `role_management`: A set of permissions that a user or an admin can do.

## Function specification

### Action proof
Contains a set of actions like post or like. When user send a transaction with any action, this contract stores their raw data along with sender and a key identifier into hashes and return them back to users. Verifier just need to know the raw data and corresponding hash then verifier can know the raw data exist.

### Role management
Contains a set of functions to manage permissions and roles.We firstly define a global role that controls every permission by setting relationships between permission controller and normal role permissions. For example, if the global role says comment permission is logic OR to normal role permission, then all community members can comment, and for logic AND it allows members in specific roles to comment. On the other hand, if global role does not have comment permission, then no one except owner can comment.

