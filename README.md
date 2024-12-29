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

**LZ4-compressed aehobak** patches yield a median reduction of **50.8%**.

**Uncompressed aehobak** patches yield a median reduction of:
- 38.1% over **LZ4-compressed bsdiff** patches
- 98.9% over **uncompressed bsdiff** patches

Direct application of aehobak patches can achieve 70% of memcpy speed, while panic-free except for out-of-memory.

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
    let patch = std::fs::read(patch_file)?;
    let mut new = Vec::new();

    aehobak::patch(&old, &patch, &mut new)?;
    std::fs::write(file, &new)
}
```
