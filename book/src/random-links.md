# Random links

- [Visualization of operational transformation with a central server](https://operational-transformation.github.io/)

## Good docs on the topic of document synchronization

- [Automerge](https://automerge.org/docs/glossary/)
- [Ethersync (a similar project)](https://ethersync.github.io/ethersync/introduction.html)
- [Ethersync decisions history (good comparisons of libraries)](https://github.com/ethersync/ethersync/tree/main/docs/decisions)

## Conflict solving libraries to try

From most promising to most obscure:

see Ethersync's comparison
[here](https://github.com/ethersync/ethersync/blob/main/docs/decisions/05-crdt-library.md)
and
[here](https://github.com/ethersync/ethersync/blob/main/docs/decisions/06-operational-transform-in-rust.md)
(they used both automerge and operational-transform)

- [Automerge](https://automerge.org/docs/hello/) +
  [Autosurgeon](https://docs.rs/autosurgeon/latest/autosurgeon/)
- [y-crdt](https://github.com/y-crdt/y-crdt) or
  [y-octo](https://github.com/y-crdt/y-octo)
- [operational-transform](https://docs.rs/operational-transform)
