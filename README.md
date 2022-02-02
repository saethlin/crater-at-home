### Crates that don't pass Miri (sorted by downloads)

| Crate | Cause | Status |
| ----- | ----- | ----- |
| smallvec-1.8.0 | Pointer invalidation via `&mut` usage | https://github.com/servo/rust-smallvec/pull/277 |
| block-buffer-0.10.0 | `&mut -> & -> &mut` | |
| crossbeam-utils-0.8.6 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| scopeguard-1.1.0 | `&mut array[index]` invalidation | |
| bytes-1.1.0 | Pointer invalidation via Box creation | https://github.com/tokio-rs/bytes/pull/523 |
| semver-1.0.4| int-to-pointer cast | https://github.com/rust-lang/unsafe-code-guidelines/issues/291
| slab-0.4.5 | `as_mut_ptr` invalidation | |
| arrayvec-0.7.2 | `as_mut_ptr` invalidation | |
| http-0.2.6 | `mem::uninitialized` | https://github.com/hyperium/http/pull/428 |
| half-1.8.2 | `&mut -> & -> &mut` | |
| crossbeam-epoch-0.9.5 | int-to-pointer cast | |
| crossbeam-deque-0.7.4 | Type validation failed in `crossbeam-epoch` | |
| prost-0.9.0 | `bytes` | See above |
| pegtraph-0.6.0 | `as_mut_ptr` invalidation | |
| rayon-1.9.1 | int-to-pointer cast | https://github.com/rayon-rs/rayon/pull/907 |
| bumpalo-3.9.1  | Many int-to-pointer casts | |
| lexical-core-0.8.1 | Please don't use this many macros to write unsafe code | |
| iovec-0.1.4 | | |
| scoped-tls-1.0.0 | | |
| arc-swap-1.5.0 | | |
| minimal-lexical-0.1.4 | | |
| aes-0.7.5 | Use of uninitialized memory in `stdarch` | |
| lru-0.7.2 | | |
| threadpool-1.8.1 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| crossbeam-0.8.1 | | |
| typed-arena-2.0.1 | Use of uninitialized memory | |
| owning_ref-0.4.1 | | |
| headers-0.3.6 | | |
| utf8-ranges-1.0.4 | Attempt to construct invalid `char` | |
| encode_unicode-0.3.6 | | |
| tracing-opentelemetry-0.16.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| beef-0.5.1 | | |
| futures-timer-3.0.2 | int-to-pointer cast (should use `AtomicPtr`) | |
| base-x-0.2.8 | | |
| rusticata-macros-4.0.0 | Incorrect `assume_init()` | |
| rusoto_credential-0.47.0 | `http`  | |
| buf_redux-0.8.4 | | |
| tokio-io-0.1.13 | Miri defect | https://github.com/rust-lang/unsafe-code-guidelines/issues/72 |
| ascii-1.0.0 | | |
| block-modes-0.8.1 | | |
| cranelift-wasm-0.80.0 | `& -> &mut` | |
| lexical-6.0.1 | `lexical-write-float` | |
| chacha20-0.8.1 | `stdarch` | |
| librocksdb-sys-6.20.3 | `bindgen` generates deref of null pointers | |
| tokio-sync-0.1.8 | Miri defect | https://github.com/rust-lang/unsafe-code-guidelines/issues/72 |
| tokio-timer-0.2.13 | Miri defect | https://github.com/rust-lang/unsafe-code-guidelines/issues/72 |
| termios-0.3.3 | Use of `mem::uninitialized` | |
| aliasable-0.1.3 | Narrowing provenance via `&mut` coercion to `*mut` | https://github.com/avitex/rust-aliasable/pull/6 |
| r2d2-0.8.9 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| tokio-current-thread-0.1.7 | Converts 0x10 into a pointer | |
| json-0.12.4 | | |
| sized-chunks-0.6.5 | | |
| asynchronous-codec-0.6.0 | `bytes` | |
| heapless-0.7.10 | | |
| rust-argon2-1.0.0 | | |
| futures-cpupool-0.1.8 | Miri defect | https://github.com/rust-lang/unsafe-code-guidelines/issues/72 |
| alloc-no-stdlib-2.0.3 | Read of uninitialized memory in test | https://github.com/dropbox/rust-alloc-no-stdlib/issues/12 |
| markup5ever-0.10.1 | `string_cache` | |
| prettytable-rs-0.8.0 | | |
| alloc-stdlib-0.2.1 | Read of uninitialized memory in test | https://github.com/dropbox/rust-alloc-no-stdlib/issues/12 |
