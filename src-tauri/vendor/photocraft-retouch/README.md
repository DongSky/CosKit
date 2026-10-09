# PhotoCraft repair kernels

Source: https://github.com/storytold/photocraft
Reference commit: ec350d64aedd019afc5eda290cc32909bc9bab7d

`src/{inpaint,poisson,retouch}.rs` are unmodified copies of
`crates/algo/src/` in the supplied PhotoCraft checkout, including upstream tests.
CosKit uses the MIT licensing option (LICENSE). The small Cargo manifest and
lib.rs isolate these CPU kernels from PhotoCraft's document and UI dependencies.
No PhotoCraft branding or artwork is included.

`src/interrupt.rs` is copied unmodified from `crates/raster/src/interrupt.rs`.
The crate aliases itself as `photocraft_raster` solely to preserve upstream
kernel references to the cancellation primitives without source modifications.
