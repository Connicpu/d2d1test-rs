# d2d1test-rs

This is a test program to demonstrate the usage of Direct2D and
DirectWrite rendering. It depends on published crates of the
stable (0.2 branch) version of [winapi-rs](https://github.com/retep998/winapi-rs),
and also the [direct2d-rs](https://github.com/Connorcpu/direct2d-rs) and
[directwrite-rs] crates that provide safe wrappers for that functionality.

This demo uses hwnd render targets, but for high performance
applications device context render targets would be better. However,
hwnd is quite a bit easier to use.

There are a few unstable features used (most significantly
[const_fn]https://github.com/rust-lang/rust/issues/24111), so for the time being,
run with nightly.
