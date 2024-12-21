# aehobak

[![GitHub](https://img.shields.io/badge/github-aehobak-ccddee?logo=github)](https://github.com/barrbrain/aehobak)
[![crates.io version](https://img.shields.io/crates/v/aehobak.svg)](https://crates.io/crates/aehobak)
[![docs.rs docs](https://docs.rs/aehobak/badge.svg)](https://docs.rs/aehobak)
[![crates.io license](https://img.shields.io/crates/l/aehobak.svg)](https://github.com/barrbrain/aehobak/blob/main/LICENSE)
[![CI build](https://github.com/barrbrain/aehobak/actions/workflows/rust.yml/badge.svg)](https://github.com/barrbrain/aehobak/actions)

Aehobak transcodes binary patches from [bsdiff](https://crates.io/crates/bsdiff).
The goal is a byte-oriented format, compact and optimised for patch application speed.
As compression efficiency is content-dependent, one should verify with a suitable corpus.
The following results are for LZ4-compressed bsdiff patches of build artifacts that are **under 3%** of the target object size. The `bench` example can report the same metrics for provided files.

**LZ4-compressed aehobak** patches are on average **45.3% smaller**.

**Uncompressed aehobak** patches are on average:
- 28.2% larger than **compressed bsdiff** patches
- 98.8% smaller than **uncompressed bsdiff** patches

## Usage

```rust
let old = vec![1, 2, 3, 4, 5];
let new = vec![1, 2, 4, 6];
let mut patch = Vec::new();
let mut encoded = Vec::new();

bsdiff::diff(&old, &new, &mut patch).unwrap();
aehobak::encode(&patch, &mut encoded).unwrap();

let mut decoded = Vec::with_capacity(patch.len());
let mut patched = Vec::with_capacity(new.len());
aehobak::decode(&mut encoded.as_slice(), &mut decoded).unwrap();
bsdiff::patch(&old, &mut decoded.as_slice(), &mut patched).unwrap();
assert_eq!(patched, new);
```

## Diffing Files

```rust
fn diff_files(orig_file: &str, file: &str, patch_file: &str) -> std::io::Result<()> {
    let old = std::fs::read(orig_file)?;
    let new = std::fs::read(file)?;
    let mut patch = Vec::new();
    let mut encoded = Vec::new();

    bsdiff::diff(&old, &new, &mut patch)?;
    aehobak::encode(&patch, &mut encoded)?;
    std::fs::write(patch_file, &encoded)
}
```

## Patching Files

```rust
fn patch_file(orig_file: &str, patch_file: &str, file: &str) -> std::io::Result<()> {
    let old = std::fs::read(orig_file)?;
    let encoded = std::fs::read(patch_file)?;
    let mut new = Vec::new();
    let mut patch = Vec::new();

    aehobak::decode(&mut encoded.as_slice(), &mut patch)?;
    bsdiff::patch(&old, &mut patch.as_slice(), &mut new)?;
    std::fs::write(file, &encoded)
}
```
