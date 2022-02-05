### Crates where `cargo miri test` flags UB (sorted by downloads)

This list maintained entirely by hand, and comes with quite a few caveats.
Most importantly, this is _not_ a list of crates which contain UB. Miri implements a prototype set of rules, and this list is based on running `cargo miri test` on the published version of each crate, with the strictest checks Miri has to offer. Some items in this list might be UB but probably won't be, some will probably be UB but technically aren't optimized on yet, and some are definitely UB right now.

If it seems like there is a crate missing, it is likely that crate's test suite attempts to execute an operation that Miri does not support.

If it seems like there's a crate here that shouldn't be, it's possible the list is out of date, the fix hasn't been published yet, or the code that causes `cargo miri test` to fail is being pulled in through a dependency. I'm attempting to call out the last case in this table, but of course this is maintained by a human so mistakes happen.

| Crate | Cause | Status |
| ----- | ----- | ----- |
| smallvec-1.8.0 | Pointer invalidation via `&mut` usage | https://github.com/servo/rust-smallvec/pull/277 |
| crossbeam-utils-0.8.6 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| scopeguard-1.1.0 | `&mut array[index]` invalidation | https://github.com/bluss/scopeguard/pull/35 |
| bytes-1.1.0 | Pointer invalidation via Box creation | https://github.com/tokio-rs/bytes/pull/523 |
| semver-1.0.4| int-to-pointer cast | https://github.com/rust-lang/unsafe-code-guidelines/issues/291
| slab-0.4.5 | `as_mut_ptr` invalidation | |
| arrayvec-0.7.2 | `as_mut_ptr` invalidation | |
| http-0.2.6 | `mem::uninitialized` | https://github.com/hyperium/http/pull/428 |
| half-1.8.2 | `&mut -> & -> &mut` | |
| crossbeam-epoch-0.9.5 | int-to-pointer cast | |
| crossbeam-deque-0.7.4 | Type validation failed in `crossbeam-epoch` | |
| prost-0.9.0 | `bytes` | |
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
| rusoto_credential-0.47.0 | `http` | |
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
| tokio-codec-0.1.2 | `bytes` 0.4.12 uses `mem::uninitialized` | |
| wasm-timer-0.2.5 | | |
| cortex-m-0.7.4 | Miri defect (access of a platform-specific address) | |
| if_rust_version-1.0.0 | Uses `mem::uninitialized` in doctest as an example | |
| malloc_buf-1.0.0 | Misaligned pointer, `*mut -> & -> &mut -> &mut` | |
| str-buf-2.0.4 | | |
| opentelemetry-http-0.6.0 | `http` | |
| android_logger-0.10.1 | Use of `mem::uninitialized` | |
| selectors-0.23.0 | `cssparser` | |
| sluice-0.5.5 | `futures-executor` | |
| rkyv-0.7.31 | | |
| ash-0.35.1+1.2.203 | `bindgen` generates deref of null pointers | |
| libp2p-gossipsub-0.35.0 | `bytes` | |
| perf-event-open-sys-1.0.1 | `bindgen` generates deref of null pointers | |
| dlmalloc-0.2.3 | | |
| field-offset-0.3.4 | | |
| thin-slice-0.1.1 | | |
| servo_arc-0.1.1 | 0x8 is not a valid pointer | |
| wgpu-core-0.12.2 | | |
| safe_arch-0.6.0 | `stdarch` | |
| bytes-utils-0.1.1 | `bytes` | |
| exit-future-0.2.0 | `futures-executor` | |
| loom-0.5.4 | `generator` | |
| generator-0.7.0 | Incorrect `assume_init()` | |
| rend-0.3.6 | Attempt to construct invalid `char` | |
| conv-0.3.3 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| hyperx-1.4.0 | `bytes` | |
| kube-core-0.67.0 | `bytes` | |
| renderdoc-sys-0.7.1 | `bindgen` generates deref of null pointers | |
| rustsec-0.25.1 | `semver` | |
| serde-json-core-0.4.0 | Use of `mem::uninitialized` | |
| aws-smithy-http-0.36.0 | `bytes` | |
| aws-sigv4-0.6.0 | `bytes` | |
| aws-smithy-async-0.36.0 | `tokio` | |
| aws-endpoint-0.6.0 | `bytes` | |
| aws-sig-auth-0.6.0 | `bytes` | |
| intervalier-0.4.0 | `futures-executor` | |
| pdqselect-0.1.1 | | |
| rustler_sys-2.1.1 | Constructing a slice from a null pointer | |
| odds-0.4.0 | reference coercion in copy | |
| wee_alloc-0.4.5 | Dereference of null pointer | |
| v_escape-0.18.0 | | |
| capnp-0.14.5 | | |
| async-graphql-value-3.0.27 | | |
| boxfnonce-0.1.1 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| rustc-rayon-core-0.3.2 | | |
| ckb-merkle-mountain-range-0.3.2 | Use of `mem::uninitialized`| |
| wasmer-2.2.0-rc1 | Wrong calling convention | |
| v_htmlescape-0.14.1 | | |
| zmq-sys-0.11.0 | `bindgen` generates deref of null pointers | |
| cgmath-0.18.0 | | |
| c_linked_list-1.1.1 | | |
| windows_reader-0.30.0 | | |
| r2d2_sqlite-0.19.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| chashmap-2.2.2 | `owning_ref` | |
| pollster-0.2.5 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| fallible_collections-0.4.4 | Construction of a too-large slice | |
| loopdev-0.4.0 | `bindgen` generates deref of null pointers | |
| freetype-0.7.0 | `bindgen` generates deref of null pointers | |
| owned-alloc-0.2.0 | | |
| smallstr-0.2.0 | | |
| rstar-0.8.4 | `pdqselect` | |
| tower-util-0.3.1 | `futures_util` | |
| tower-limit-0.1.3 | `tokio-sync` | |
| tower-buffer-0.3.0 | `tokio` | |
| tower-retry-0.3.0 | `tokio` | |
| aligned-0.4.0 | | |
| supercow-0.1.0 | Use of `mem::uninitialized` | |
| coarsetime-0.1.21 | Incorrect `assume_init()` | |
| pkcs11-0.5.0 | | |
| bb8-0.7.1 | `tokio` | |
| quinn-proto-0.8.0 | `bytes` | |
| primal-check-0.3.1 | Offsetting a pointer out of bounds | |
| rulinalg-0.4.2 | Use of `mem::uninitialized` | |
| intrusive-collections-0.9.3 | | |
| lru_time_cache-0.11.11 | | |
| binary-heap-plus-0.4.1 | | |
| httptest-0.15.4 | `bytes` | |
| http-serde-1.0.3 | `http` | |
| r0-1.0.0 | | |
| spinning-0.1.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| packed_struct-0.10.0 | `bitvec` | |
| jod-thread-0.1.2 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| memory-lru-0.1.0 | `lru` | |
| platform-info-0.2.0 | Use of `mem::uninitialized` | |
| memmem-0.1.1 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| futures-locks-0.7.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| blake2b-rs-0.2.0 | `bindgen` generates deref of null pointers | |
| typed-index-collections-3.0.3 | | |
| rustc-ap-rustc_index-727.0.0 | `arrayvec` | |
| axum-core-0.1.1 | `http` | |
| rustc-ap-rustc_arena-727.0.0 | | |
| aws-smithy-eventstream-0.36.0 | `bytes` | |
| triggered-0.1.2 | | |
| wasm-bindgen-wasm-interpreter-0.2.79 | `& -> &mut` | |
| safe-transmute-0.11.2 | misaligned pointer | |
| libparted-sys-0.3.1 | `bindgen` generates deref of null pointers | |
| ndarray-stats-0.5.0 | `matrixmultiply` | |
| crc64-1.0.0 | misaligned pointer | |
| randomkit-0.1.1 | Use of `mem::uninitialized` | |
| usb-device-0.2.8 | Use of `mem::uninitialized` | |
| versionize-0.1.6 | `crc64` | |
| ustr-0.8.1 | | |
| blkid-sys-0.1.6 | `bindgen` generates deref of null pointers | |
| jwt-simple-0.10.8 | `coarsetime` | |
| blst-0.3.6 | Incorrect `assume_init()` | |
| futures_codec-0.4.1 | `bytes` | |
| tonic-web-0.2.0 | `bytes` | |
| circular-0.3.0 | `ptr::copy` slicing invalidation | |
| json-rpc-types-1.0.2 | `str-buf` | |
| derive-error-0.0.5 | `rayon` | |
| typed-headers-0.2.0 | `bytes` | |
| timer-0.2.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| mqttbytes-0.6.0 | `bytes` | |
| abomonation_derive-0.5.0 | `abomonation` | |
