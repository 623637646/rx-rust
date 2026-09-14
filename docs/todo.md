# TODOs

1. Make `Observable`'s `T` and `E` associated types instead of type parameters. rustc currently
   hangs on that change, see [issue](https://github.com/rust-lang/rust/issues/159051); revisit after upgrading the compiler.
   Note: even once they are associated types, they will not be turned into a borrowing
   `Item<'a>` GAT — see [decisions/0001-no-borrowed-items.md](decisions/0001-no-borrowed-items.md).
