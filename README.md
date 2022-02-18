### Crates where `cargo miri test` flags UB (approximately sorted by downloads)

This list maintained entirely by hand, and comes with quite a few caveats.
Most importantly, this is _not_ a list of crates which contain UB. Miri implements a prototype set of rules, and this list is based on running `cargo miri test` on the published version of each crate, with the strictest checks Miri has to offer. Some items in this list might be UB but probably won't be, some will probably be UB but aren't yet, and some are definitely UB right now and may produce miscompilations. It is a goal for this list to eventually be empty, but that will mostly likely occur due to a combination of changes to the rules that Miri checks and patches to the listed crates.

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
| crossbeam-epoch-0.9.7 | int-to-pointer cast | |
| crossbeam-deque-0.7.4 | Type validation failed in `crossbeam-epoch` | https://github.com/crossbeam-rs/crossbeam/pull/779 |
| gimli-0.26.1 | `as_ptr -> Vec` | https://github.com/gimli-rs/gimli/pull/614 |
| prost-0.9.0 | `bytes` | |
| pegtraph-0.6.0 | `as_mut_ptr` invalidation | |
| rayon-1.9.1 | int-to-pointer cast | https://github.com/rayon-rs/rayon/pull/907 |
| bumpalo-3.9.1  | Many int-to-pointer casts | |
| lexical-core-0.8.1 | Please don't use this many macros to write unsafe code | |
| bitvec-1.0.0 | | |
| iovec-0.1.4 | `& -> &mut` | Fixed version was yanked |
| scoped-tls-1.0.0 | int-to-pointer cast | https://github.com/rust-lang/unsafe-code-guidelines/issues/291 |
| arc-swap-1.5.0 | Provenance + mutation through a pointer derived from `&T` | |
| minimal-lexical-0.1.4 | | |
| aes-0.7.5 | Use of uninitialized memory in `stdarch` | |
| lru-0.7.2 | invalidating a pointer by moving the Box it points into | |
| threadpool-1.8.1 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| crossbeam-0.8.1 | `crossbeam-epoch` | |
| typed-arena-2.0.1 | pointer invalidation from writes | |
| owning_ref-0.4.1 | invalidating a pointer by moving the Box it points into | |
| headers-0.3.6 | `bytes` | |
| utf8-ranges-1.0.4 | Attempt to construct invalid `char` | |
| encode_unicode-0.3.6 | Invalid get_unchecked | https://github.com/tormol/encode_unicode/issues/12 |
| tracing-opentelemetry-0.16.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| beef-0.5.1 | | |
| futures-timer-3.0.2 | int-to-pointer cast (should use `AtomicPtr`) | |
| base-x-0.2.8 | | |
| lalrpop-0.19.7 | | |
| rusoto_credential-0.47.0 | `http` | |
| buf_redux-0.8.4 | | |
| tokio-io-0.1.13 | Miri defect | https://github.com/rust-lang/unsafe-code-guidelines/issues/72 |
| ascii-1.0.0 | | |
| block-modes-0.8.1 | | |
| twox-hash | `stdarch` | |
| cranelift-wasm-0.80.0 | `& -> &mut` | |
| lexical-6.0.1 | `lexical-write-float` | |
| chacha20-0.8.1 | `stdarch` | |
| librocksdb-sys-6.20.3 | `bindgen` generates deref of null pointers | |
| tokio-sync-0.1.8 | Miri defect | https://github.com/rust-lang/unsafe-code-guidelines/issues/72 |
| tokio-timer-0.2.13 | Miri defect | https://github.com/rust-lang/unsafe-code-guidelines/issues/72 |
| termios-0.3.3 | Use of `mem::uninitialized` | |
| aliasable-0.1.3 | Narrowing provenance via `&mut` coercion to `*mut` | https://github.com/avitex/rust-aliasable/pull/6 |
| r2d2-0.8.9 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| rgb-0.8.31 | `& -> &mut` | |
| tokio-current-thread-0.1.7 | Converts 0x10 into a pointer | |
| json-0.12.4 | Self-referential structure | |
| sized-chunks-0.6.5 | | |
| asynchronous-codec-0.6.0 | `bytes` | |
| heapless-0.7.10 | | |
| futures-intrusive-0.4.0 | | |
| rust-argon2-1.0.0 | | |
| futures-cpupool-0.1.8 | Miri defect | https://github.com/rust-lang/unsafe-code-guidelines/issues/72 |
| alloc-no-stdlib-2.0.3 | Read of uninitialized memory in test | https://github.com/dropbox/rust-alloc-no-stdlib/issues/12 |
| markup5ever-0.10.1 | `string_cache` | |
| prettytable-rs-0.8.0 | | |
| alloc-stdlib-0.2.1 | Read of uninitialized memory in test | https://github.com/dropbox/rust-alloc-no-stdlib/issues/12 |
| tokio-codec-0.1.2 | `bytes` 0.4.12 uses `mem::uninitialized` | |
| attohttpc-0.18.0 | `http` | |
| wasm-timer-0.2.5 | | |
| cortex-m-0.7.4 | Miri defect (access of a platform-specific address) | |
| if_rust_version-1.0.0 | Uses `mem::uninitialized` in doctest as an example | |
| malloc_buf-1.0.0 | Misaligned pointer, `*mut -> & -> &mut -> &mut` | |
| str-buf-2.0.4 | | |
| opentelemetry-http-0.6.0 | `http` | |
| android_logger-0.10.1 | Use of `mem::uninitialized` | |
| selectors-0.23.0 | `cssparser` | |
| sluice-0.5.5 | | |
| rkyv-0.7.31 | | |
| ash-0.35.1+1.2.203 | `bindgen` generates deref of null pointers | |
| libp2p-relay-0.6.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| libp2p-gossipsub-0.35.0 | `bytes` | |
| perf-event-open-sys-1.0.1 | `bindgen` generates deref of null pointers | |
| dlmalloc-0.2.3 | | |
| field-offset-0.3.4 | | |
| thin-slice-0.1.1 | | |
| servo_arc-0.1.1 | 0x8 is not a valid pointer | |
| wgpu-core-0.12.2 | | |
| safe_arch-0.6.0 | `stdarch` | |
| bytes-utils-0.1.1 | `bytes` | |
| sled-0.34.7 | | |
| loom-0.5.4 | `generator` | |
| generator-0.7.0 | Incorrect `assume_init` | |
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
| intervalier-0.4.0 | ICE? | |
| pdqselect-0.1.1 | | |
| rustler_sys-2.1.1 | Constructing a slice from a null pointer | |
| odds-0.4.0 | reference coercion in copy | |
| wee_alloc-0.4.5 | Dereference of null pointer | |
| v_escape-0.18.0 | | |
| capnp-0.14.5 | | |
| async-graphql-value-3.0.27 | | |
| boxfnonce-0.1.1 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| rustc-rayon-core-0.3.2 | | |
| ckb-merkle-mountain-range-0.3.2 | Use of `mem::uninitialized` | |
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
| coarsetime-0.1.21 | Incorrect `assume_init` | |
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
| ustr-0.8.1 | int-to-pointer cast | |
| blkid-sys-0.1.6 | `bindgen` generates deref of null pointers | |
| jwt-simple-0.10.8 | `coarsetime` | |
| blst-0.3.6 | Incorrect `assume_init` | |
| futures_codec-0.4.1 | `bytes` | |
| tonic-web-0.2.0 | `bytes` | |
| circular-0.3.0 | `ptr::copy` slicing invalidation | |
| json-rpc-types-1.0.2 | `str-buf` | |
| derive-error-0.0.5 | `rayon` | |
| typed-headers-0.2.0 | `bytes` | |
| timer-0.2.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| mqttbytes-0.6.0 | `bytes` | |
| abomonation_derive-0.5.0 | `abomonation` | |
| aws_lambda_events-0.5.0 | `http` | |
| wasmer-near-2.2.0 | Wrong calling convention | |
| kvm-ioctls-0.11.0 | Miri defect | https://github.com/rust-lang/miri/issues/1892 |
| abomonation-0.7.3 | So many things. Please don't use this crate. | |
| swc_ecma_utils-0.64.0 | `string_cache` | |
| grep-printer-0.1.6 | `stdarch` | |
| tower-load-0.3.0 | `tokio` | |
| dprint-swc-ecma-ast-view-0.48.2 | `bumpalo` | |
| dprint-core-0.50.0 | `bumpalo` | |
| tauri-utils-1.0.0-beta.3 | `html5ever` | |
| triomphe-0.1.5 | | |
| cryptoxide-0.4.1 | | |
| futures-batch-0.6.0 | `tokio` | |
| cmac-0.6.0 | `stdarch` | |
| bevy-glsl-to-spirv-0.2.1 | `bindgen` generates deref of null pointers | |
| linked_list_allocator-0.9.1 | | |
| nvml-wrapper-sys-0.5.0 | `bindgen` generates deref of null pointers | |
| garando_syntax-0.1.0 | Out of bounds `get_unchecked` | |
| dprint-plugin-typescript-0.62.1 | `string_cache` | |
| ritelinked-0.3.2 | | |
| ckb-librocksdb-sys-6.20.4 | `bindgen` generates deref of null pointers | |
| csaps-0.3.0 | | |
| capnp-futures-0.14.1 | `futures_executor` | |
| miette-3.3.0 | Invalid transmute, like `color-eyre` | |
| tentacle-multiaddr-0.3.2 | `bytes` | |
| crypto_api_chachapoly-0.5.0 | `json` | |
| unzip-0.1.0 | `temporary` | |
| tower-ready-cache-0.3.1 | `futures_util` | |
| minisign-verify-0.2.0 |
| lsp-codec-0.3.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| www-authenticate-0.4.0 | `bytes` | |
| parse_link_header-0.3.2 | `bytes` | |
| vtparse-0.6.0 | Out-of-bounds `offset` | |
| arraydeque-0.4.5 | | |
| hamming-0.1.3 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| sxd-document-0.3.2 | Attempt to create a dangling reference | |
| thread-scoped-1.0.2 | | |
| smallbitvec-2.5.1 | | |
| bloom-filters-0.1.2 | misaligned pointer | |
| fluvio-wasm-timer-0.2.5 | | |
| virtio-bindings-0.1.0 | `bindgen` generates deref of null pointers | |
| hibitset-0.6.3 | | |
| thunderdome-0.5.0 | | |
| datatest-stable-0.1.1 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| tokio-proto-0.1.1 | Depends on an ancient and buggy version of `smallvec` | |
| fasthash-sys-0.3.2 | `bindgen` generates deref of null pointers | |
| aws-sdk-secretsmanager-0.6.0 | `http` | |
| syscalls-0.5.0 | Miri defect | https://github.com/rust-lang/miri/pull/1970 |
| selinux-sys-0.5.1 | `bindgen` generates deref of null pointers | |
| atom-0.3.6 | | |
| fts-sys-0.2.1 | `bindgen` generates deref of null pointers | |
| qutex-0.2.3 | Miri defect | https://github.com/rust-lang/unsafe-code-guidelines/issues/72 |
| base58check-0.1.0 | `generic-array` used to use `mem::uninitialized` | |
| bus-2.2.3 | | |
| boring-sys-2.0.0 | `bindgen` generates deref of null pointers | |
| tryhard-0.4.0 | `tokio` | |
| flurry-0.3.1 | `crossbeam_epoch` | |
| triple_accel-0.4.0 | | |
| vfio-bindings-0.3.1 | `bindgen` generates deref of null pointers | |
| serde_tokenstream-0.1.3 | | |
| primordial-0.4.0 | | |
| relay-0.1.1 | Miri defect | https://github.com/rust-lang/unsafe-code-guidelines/issues/72 |
| jwalk-0.6.0 | `crossbeam_epoch` | |
| serde-hex-0.1.0 | Depends on an old version of array-init which uses `mem::uninitialized` | |
| rust_hawktracer_sys-0.4.2 | `bindgen` generates deref of null pointers | |
| harfbuzz-sys-0.5.0 | `bindgen` generates deref of null pointers | |
| ndarray_einsum_beta-0.7.0 | `matrixmultiply` | |
| tame-gcs-0.10.0 | `bytes` | |
| addr-0.15.3 | `rayon` | |
| ferris-says-0.2.1 | Depends on an old and broken version of `smallvec` | |
| reqwest-tracing-0.2.0 | `http` | |
| crossfire-0.1.7 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| avahi-sys-0.10.0 | `bindgen` generates deref of null pointers | |
| ffmpeg-sys-next-4.4.0-next.2 | `bindgen` generates deref of null pointers | |
| primal-estimate-0.3.1 | Out-of-bounds `offset` | |
| cosmwasm-bignumber-2.2.0 | `bigint` | |
| httpmock-0.6.6 | `bytes` | |
| mft-0.6.0 | `lru` | |
| minimp3-sys-0.3.2 | `bindgen` generates deref of null pointers | |
| internment-0.5.5 | | |
| tokio-process-0.2.5 | Old version of `crossbeam-queue` | |
| sprs-0.11.0 | `matrixmultiply` | |
| rust-s3-0.28.1 | `http` | |
| leaky-bucket-0.11.0 | `tokio` | |
| elf_rs-0.2.0 | Misaligned pointer | |
| rumqttc-0.10.0 | `bytes` | |
| xmas-elf-0.8.0 | Transmute to an unaligned reference | |
| dhat-0.3.0 | Miri thinks there is deallocation of Rust heap with `libc::free` | |
| rav1e-0.5.1 | `rayon` | |
| deno_lint-0.23.0 | `scoped-tls` | |
| bcder-0.6.1 | | |
| libxml-0.3.0 | `bindgen` generates deref of null pointers | |
| httpbis-0.9.1 | `bytes` | |
| azure_storage_mirror-1.0.0 | `bytes` | |
| primal-0.3.0 | Out-of-bounds `offset` | |
| leaky-bucket-lite-0.5.1 | `tokio` | |
| near-vm-logic-0.11.0 | | |
| mpart-async-0.5.0 | `bytes` | |
| shiplift-0.7.0 | `bytes` | |
| lopdf-0.27.0 | Old `itoa` used `mem::uninitialized` | |
| openat-0.1.21 | Miri defect | https://github.com/rust-lang/miri/pull/1970 |
| tun-0.5.3 | Miri defect | https://github.com/rust-lang/miri/pull/1970 |
| mdbook-linkcheck-0.7.6 | `http` | |
| deno_ast-0.10.0 | `string_cache` | |
| shred-0.12.0 | `crossbeam_epoch` | |
| tantivy-0.16.1 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| resize-0.7.2 | `rgb` | |
| cargo-update-8.1.2 | `json` | |
| octocrab-0.15.4 | `bytes` | |
| serde_prometheus-0.1.6 | Old `itoa` used `mem::uninitialized` | |
| bevy_tasks-0.6.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| select-0.6.0-alpha.1 | `tendril` | |
| adblock-0.4.3 | `seahash` | |
| metrics-runtime-0.13.1 | `crossbeam_epoch` | |
| ark-groth16-0.3.0 | `rayon` | |
| ws-0.9.1 | `generic-array` | |
| io-uring-0.5.2 | `bindgen` generates deref of null pointers | |
| dmsort-1.0.1 | | |
| psl-2.0.68 | `rayon` | |
| r2d2_redis-0.14.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| protobuf-parse-3.0.0-alpha.2 | `linked_hash_map` | |
| rustdct-0.7.0 | `rustfft` | |
| libusb1-sys-0.6.0 | Incorrect `assume_init` | |
| elementtree-0.7.0 | `string_cache` | |
| prettydiff-0.5.1 | `prettytable` | |
| nom_locate-4.0.0 | | |
| bigint-4.4.3 | Use of `mem::uninitialized` | |
| rusb-0.9.0 | | |
| ringbuf-0.2.6 | | |
| boring-2.0.0 | Use of `mem::uninitialized` | |
| scraper-0.12.0 | `tendril` | |
| sdl2-sys-0.35.1 | `bindgen` generates deref of null pointers | |
| remoteprocess-0.4.8 | | |
| geo-0.18.0 | `pdqselect` | |
| grep-searcher-0.1.8 | `bstr` | |
| kuchiki-0.8.1 | `html5ever` | |
| rustc-rayon-0.3.2 | | |
| xcb-1.0.0-beta.4 | Misaliged pointer | |
| hyperlocal-0.8.0 | `bytes` | |
| ammonia-3.1.3 | `tendril` | |
| rusoto_dynamodb-0.47.0 | `bytes` | |
| markup5ever_rcdom-0.1.0 | `html5ever` | |
| metrics-util-0.11.0 | | |
| jsonrpc-http-server-18.0.0 | `http` | |
| xml5ever-0.16.2 | `markup5ever` | |
| generator-0.7.0 | Incorrect `assume_init` | |
| async-tungstenite-0.16.1 | `bytes` | |
| toml_edit-0.13.3 | `rayon` | |
| color-eyre-0.6.0 | | |
| protobuf-codegen-3.0.0-alpha.2 | `get_unchecked` out of bounds | |
| tendril-0.4.2 | int-to-pointer cast | |
| ndarray-0.15.4 | Narrowing provenance through a reference | |
| integer-encoding-2.0.2 | Misaligned pointer | |
| pretty-0.11.2 | | |
| matrixmultiply-0.3.2 | | |
| backoff-0.4.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| blocking-1.1.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| string_cache-0.8.2 | Invalidating a pointer by moving the Box it points into | |
| tungstenite-0.16.0 | `http` | |
| hostname-0.3.1 | `syn` | |
| h2-0.3.11 | `http` | |
| hyper-0.14.16 | `bytes` | |
| crossbeam-epoch-0.9.6 | int-to-pointer cast | |
| vsdbsled-0.34.7-patched | | |
| trees-0.4.2 | | |
| context-2.1.0 | Use of `mem::uninitialized` | |
| dprint-plugin-markdown-0.12.1 | `bumpalo` | |
| realfft-2.0.1 | | |
| spmc-0.3.0 | | |
| pkg-version-1.0.0 | `syn` | |
| arraystring-0.3.0 | | |
| grpc-0.8.3 | `bytes` | |
| svgdom-0.18.0 | `siphasher` | |
| tame-oauth-0.6.0 | `bytes` | |
| itsdangerous-0.4.0 | `generic-array` | |
| enr-0.5.1 | `bytes` | |
| sp-utils-3.0.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| linkcheck-0.4.1 | `bytes` | |
| deno_graph-0.21.1 | `string_cache` | |
| polars-core-0.19.1 | | |
| patricia_tree-0.3.1 | | |
| slice-group-by-0.3.0 | | |
| rapier2d-0.12.0-alpha.1 | | |
| sqlite3-sys-0.13.0 | `temporary` | |
| miniz_oxide_c_api-0.3.0 | | |
| domain-0.6.1 | `as_ptr -> &mut` | |
| crt0stack-0.1.0 | | |
| lambda_http-0.4.1 | `http` | |
| order-stat-0.1.3 | | |
| target_build_utils-0.3.1 | `siphasher` | |
| self_encryption-0.27.1 | `crossbeam_epoch` | |
| dssim-core-3.1.0 | `rayon` | |
| html2md-0.2.13 | `tendril` | |
| rust-gmp-kzen-0.5.1 | Use of `mem::uninitialized` | |
| flatdata-0.5.3 | | |
| dssim-3.1.2 | `rayon` | |
| kvm-bindings-0.5.0 | `bindgen` generates deref of null pointers | |
| rapier3d-0.12.0-alpha.1 | | |
| cloudflare-0.9.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| bbs-0.4.1 | `generic-array` | |
| mbox-0.6.0 | | |
| tracy-client-sys-0.16.0 | `bindgen` generates deref of null pointers | |
| lenient_semver-0.4.2 | `semver` | |
| yastl-0.1.2 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| tract-linalg-0.15.8 | Out-of-bounds `offset` | |
| stun_codec-0.1.13 | | |
| croaring-sys-0.5.1 | `bindgen` generates deref of null pointers | |
| libevent-sys-0.2.4 | `bindgen` generates deref of null pointers | |
| paho-mqtt-sys-0.6.0 |  `bindgen` generates deref of null pointers | |
| lioness-0.1.2 | `generic-array` | |
| epimetheus-0.7.1 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| sxd-xpath-0.4.2 | | |
| cached-path-0.5.1 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| drain-0.1.0 | `tokio` | |
| memcache-0.16.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| hyperscan-sys-0.2.2 | `bindgen` generates deref of null pointers | |
| cargo-hakari-0.9.11 | `color_eyre` | |
| minisign-0.7.0 | | |
| av-data-0.3.0 | `bytes` | |
| stable-eyre-0.2.2 | `eyre` | |
| storage-proofs-core-11.0.0 | `generic-array` | |
| near-sdk-4.0.0-pre.6 | | |
| murmurhash3-0.0.5 | transmuting a `&[u8]` to a `&[u64]`, producing a dangling reference | |
| tensorflow-sys-0.20.0 | `bindgen` generates deref of null pointers | |
| moka-cht-0.4.2 | | |
| curv-kzen-0.9.0 | Use of `mem::uninitialized` | |
| cosmrs-0.4.1 | `bytes` | |
| pen-ffi-0.3.39 | | |
| skim-0.9.4 | `beef` | |
| cuda-driver-sys-0.3.0 | `bindgen` generates deref of null pointers | |
| sw-composite-0.7.14 | `stdarch` | |
| lua_bind_hash-1.0.1 | Misaligned pointer | |
| mp4parse-0.12.0 | | |
| vek-0.15.5 | Incorrect use of `assume_init` | |
| dodrio-0.2.0 | `bumpalo` | |
| xkbcommon-sys-1.3.0 | `bindgen` generates deref of null pointers | |
| hashicorp_vault-2.1.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| reddsa-0.2.0 | `bitvec` | |
| rand_seeder-0.2.2 | copy+paste of old `siphash` code | |
| metrohash-1.0.6 | Use of `mem::uninitialized` | |
| userfaultfd-sys-0.4.1 | `bindgen` generates deref of null pointers | |
| elastic-array-plus-0.10.0 | Use of `mem::uninitialized` | |
| rtp-0.6.5 | `bytes` | |
| redis-protocol-4.0.1 | `bytes` | |
| sidekiq-0.11.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| timely_communication-0.12.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| ratelimit_meter-5.0.0 | | |
| blake2b-ref-0.3.0 | Unaligned reference | |
| nipper-0.1.9 | `tendril` | |
| rtcp-0.6.5 | `bytes` | |
| async_cell-0.2.0 | `generator` | |
| webrtc-media-0.4.5 | `bytes` | |
| rusty_pool-0.6.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| imap-3.0.0-alpha.4 | | |
| html2text-0.2.1 | `html4ever` | |
| xoodyak-0.7.3 | `stdarch` | |
| near-sdk-core-3.1.0 | `syn` | |
| deadqueue-0.2.0 | `tokio` | |
| zeno-0.2.2 | Incorrect use of `assume_init` | |
| stack-buf-0.1.6 | | |
| ntex-router-0.5.1 | `bytes` | |
| conrod_core-0.76.1 | `petgraph` | |
| fluvio-0.12.1 | `bytes` | |
| devicemapper-sys-0.1.2 | `bindgen` generates deref of null pointers | |
| flapigen-0.6.0-pre7 | `petgraph` | |
| rust-releases-0.21.1 | `http` | |
| splitmut-0.2.1 | | |
| dynfmt-0.1.5 | Incorrect use of `assume_init` | |
| tract-hir-0.15.8 | Creating a slice from a null pointer | |
| buddy-alloc-0.4.1 | | |
| ntex-bytes-0.1.11 | Incorrect use of `assume_init` | |
| libcryptsetup-rs-0.4.4 | `strlen` called on zero-size allocation | |
| shuffling-allocator-1.1.2 | | |
| aligned_alloc-0.1.3 | Invalid alignment passed to `posix_memalign`? Maybe? | |
| aws-sdk-kinesis-0.6.0 | `http` | |
| starlark-0.6.0 | | |
| libcryptsetup-rs-sys-0.1.6 | `bindgen` generates deref of null pointers | |
| indexed-0.2.0 | Misaligned pointer | |
| aws-sdk-sso-0.6.0 | `http` | |
| pinboard-2.1.0 | `crossbeam_epoch` | |
| dockworker-0.0.23 | `bytes` | |
| refpool-0.4.3 | | |
| nav-types-0.5.1 | `nalgebra` | |
| sliceslice-0.3.1 | `stdarch` | |
| merkle-sha3-0.1.0 | `rust-crypto` | |
| mundane-0.5.0 | `bindgen` generates deref of null pointers | |
| legion-0.4.0 | `smallvec` | |
| ed25519-bip32-0.4.1 | `cryptoxide` | |
| teloxide-core-0.3.4 | `bytes` | |
| amethyst_network-0.15.3 | `bytes` | |
| rqrr-0.4.0 | `lru` | |
| ref_filter_map-1.0.1 | | |
| checkers-0.6.0 | Use of uninitialized memory | |
| mozangle-0.3.3 | `bindgen` generates deref of null pointers | |
| buddy_system_allocator-0.8.0 | | |
| libblkid-rs-sys-0.1.3 | `bindgen` generates deref of null pointers | |
| deque-0.3.2 | | |
| v4l-0.12.1 | Miri defect | https://github.com/rust-lang/miri/pull/1970 |
| os_socketaddr-0.2.0 | Misaligned pointer | |
| v_jsonescape-0.6.1 | | |
| rasn-0.5.0 | `bitvec` | |
| ffmpeg-sys-4.3.3 | `bindgen` generates deref of null pointers | |
| imagequant-sys-4.0.0-beta.9 | Deref of dangling pointer | |
| vdso-0.2.0 | Out-of-bounds offset | |
| twapi-oauth-0.1.4 | `crypto` | |
| pam-sys-1.0.0-alpha4 | `bindgen` generates deref of null pointers | |
| defer-drop-1.2.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| wrangler-1.19.7 | `http` | |
| reductive-0.9.0 | `matrixmultiply` | |
| ittapi-rs-0.1.6 | `bindgen` generates deref of null pointers | |
| failsafe-1.1.0 | | |
| v4l2-sys-mit-0.2.0 | `bindgen` generates deref of null pointers | |
| aws-sdk-dynamodb-0.6.0 | `http` | |
| rpki-0.14.1 | `bytes` | |
| gitlab-0.1407.0 | `bytes` | |
| thin-vec-0.2.4 | | |
| ref_thread_local-0.1.1 | | |
| ink_primitives-3.0.0-rc8 | Misaligned pointer | |
| ink_env-3.0.0-rc8 | `ink_primitives` | |
| libxlsxwriter-sys-1.1.1 | `bindgen` generates deref of null pointers | |
| bee-crypto-0.3.0 | `bee_ternary` | |
| elrond-wasm-0.27.2 | calling `alloc::alloc` with size 0 | |
| cargo-deadlinks-0.8.1 | `cssparser`/`selectors` | |
| webpage-1.4.0 | `html5ever` | |
| bayer-0.1.5 | Misaligned slice | |
| gifski-1.6.4 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| glsl-layout-0.4.0 | `offset_of!` expands to deref of null pointers | |
| stable_bst-0.2.0 | | |
| scoped-pool-1.0.0 | Old `crossbeam` used `mem::uninitialized` | |
| selenium-rs-0.1.2 | Old `bytes` used `mem::uninitialized` | |
| tract-tensorflow-0.15.8 | `tract-linalg` | |
| gc-0.4.1 | | |
| datadog-apm-sync-0.4.4 | `http` | |
| elrond-wasm-debug-0.27.2 | `rand_seeder` | |
| dynomite-0.10.0 | `bytes` | |
| riker-0.4.2 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| slice-pool-0.4.1 | | |
| strings-0.1.1 | | |
| dipstick-0.9.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| lazy-init-0.5.0 | Old version of `crossbeam-sync` used `mem::uninitialized` | |
| couchbase-sys-1.0.0-alpha.4 | `bindgen` generates deref of null pointers | |
| http-bytes-0.1.0 | Old version of `bytes` | |
| august-2.4.0 | `tendril` | |
| git-url-parse-0.4.0 | `eyre` | |
| multiboot-0.7.0 | | |
| fasteval-0.2.4 | | |
| collision-0.20.1 | `smallvec` | |
| toolshed-0.8.1 | | |
| rumqttlog-0.9.0 | `bytes` | |
| hubcaps-0.6.2 | `http` | |
| ws_stream_tungstenite-0.7.0 | `ringbuf` | |
| nebari-0.2.1 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| libiio-sys-0.3.1 | `bindgen` generates deref of null pointers | |
| sauron-0.43.10 | `sauron-core` | |
| slack-hook-0.8.0 | Old version of `bytes` | |
| pelite-0.9.0 | Use of `mem::uninitialized` | |
| colosseum-0.2.2 | | |
| lalrpop-snap-0.16.0 | `siphasher` | |
| jetscii-0.5.1 | | |
| rust-gmp-0.5.0 | Use of `mem::uninitialized` | |
| nanoserde-0.1.29 | Misaligned pointer | |
| cuda-runtime-sys-0.3.0-alpha.1 | `bindgen` generates deref of null pointers | |
| proxy-protocol-0.5.0 | `bytes` | |
| ryu-ecmascript-0.1.1 | | |
| rust_tokenizers-7.0.1 | `rayon` | |
| ark-poly-commit-0.3.0 | `rayon` | |
| sha1collisiondetection-0.2.5 | Incorrect use of `assume_init` | |
| k-0.27.0 | `nalgebra` | |
| hyper_serde-0.12.0 | `http` | |
| libR-sys-0.2.2 | `bindgen` generates deref of null pointers | |
| r2d2-memcache-0.6.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| dmidecode-0.7.1 | | |
| hts-sys-2.0.2 | `bindgen` generates deref of null pointers | |
| differential-dataflow-0.12.0 | | |
| kira-0.6.0-beta.5 | `ringbuf` | |
| twilight-http-ratelimiting-0.9.0 | `http` | |
| clipper-sys-0.7.1 | `bindgen` generates deref of null pointers | |
| umash-sys-0.2.0 | `bindgen` generates deref of null pointers | |
| sputnikvm-0.11.0-beta.0 | `generic-array` | |
| micro-timer-0.4.0 | Deref of `0x1` | |
| simd-json-derive-0.2.2 | Old `itoa` uses `mem::uninitialized` | |
| nalgebra-mvn-0.12.0 | `nalgebra` | |
| bpf-sys-2.3.0 | `bindgen` generates deref of null pointers | |
| nsvg-0.5.1 | `bindgen` generates deref of null pointers | |
| hsl-0.1.1 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| nettle-sys-2.0.8 | `bindgen` generates deref of null pointers | |
| wedpr_l_crypto_hash_keccak256-1.1.0 | `generic-array` | |
| apriltag-sys-0.2.0 | `bindgen` generates deref of null pointers | |
| ijson-0.1.3 | | |
| sparse-merkle-tree-0.5.3 | Old version of `blake2b-rs` | |
| webrtc-vad-0.4.0 | `bindgen` generates deref of null pointers | |
| http-zipkin-0.3.0 | `http` | |
| aws-sdk-eks-0.6.0 | `http`| |
| mbrman-0.4.2 | `bitvec` | |
| tokio-dns-unofficial-0.4.0 | `futures` | |
| psa-crypto-sys-0.9.1 | `bindgen` generates deref of null pointers | |
| jss-0.4.0 | `json` | |
| slice_as_array-1.1.0 | Use of `mem::uninitialized` | |
| thirtyfour_sync-0.27.1 | `bytes` | |
| libquickjs-sys-0.10.0 | `bindgen` generates deref of null pointers | |
| circulate-0.2.1 | | |
| ttaw-0.3.0 | `bytes` | |
| fuse-0.3.1 | Misaligned pointer | |
| aws-sdk-ssm-0.6.0 | `http` | |
| rsass-0.23.0 | `nom_locate` | |
| hdf5-types-0.8.1 | Misaligned pointer | |
| pulse-0.5.3 | | |
| r2d2_sqlite_neonphog-0.18.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| ruma-api-0.18.5 | Old version of `itoa` | |
| meilisearch-sdk-0.13.0 | `bytes` | |
| glean-43.0.2 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| gdnative-core-0.9.3 | | |
| gdnative-sys-0.9.3 | `bindgen` generates deref of null pointers | |
| coco-0.3.4 | | |
| varisat-dimacs-0.2.2 | Old version of `itoa` | |
| maligned-0.2.1 | Incorrect layout on deallocation | |
| markup-0.12.5 | Old version of `itoa` | |
| yarte_helpers-0.15.6 | | |
| libappindicator-sys-0.7.0 | `bindgen` generates deref of null pointers | |
| ruma-serde-0.5.0 | Old version of `itoa` | |
| rexsgdata-0.12.0 | `bytes` | |
| rasn-snmp-0.5.0 | `bytes` | |
| concread-0.2.21 | `ptr::copy` invalidation pattern | |
| double-checked-cell-2.1.0 | Old version of `crossbeam` | |
| redbpf-2.3.0 | `bindgen` generates deref of null pointers | |
| ruma-client-api-0.12.3 | `bytes` | |
| varisat-0.2.2 | | |
| compression-0.1.5 | | |
| mp4-0.9.2 | `bytes` | |
| transaction-pool-2.0.3 | | |
| jql-3.0.8 | `rayon` | |
| barrage-0.2.1 | `tokio` | |
| stackvector-1.1.1 | | |
| rust_cascade-0.6.0 | `murmurhash3` | |
| poldercast-1.2.1 | `cryptoxide` | |
| ndarray-csv-0.5.1 | Old version of `itoa` | |
| str-concat-0.2.0 | | |
| fixed-slice-vec-0.8.0 | | |
| html2pango-0.4.1 | `html5ever` | |
| html5ever-atoms-0.3.0 | `siphasher` | |
| thin-dst-1.1.0 | | |
| par-map-0.1.4 | `futures` | |
| psl-lexer-0.3.1 |` rayon` | |
| prost-amino-0.6.0 | `bytes` | |
| elrond-wasm-output-0.27.2 | `wee_alloc` | |
| lexpr-0.2.6 | Old version of `itoa` | |
| alloc_counter-0.0.4 | Miri defect (Miri gets libc and System allocator confused) | |
| controlled-option-0.4.1 | Use of uninitialized memory | |
| tectonic-0.8.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| deku-0.12.5 | `bitvec` | |
| json_in_type-1.1.1 | Use of `mem::uninitialized` | |
| bcc-sys-0.23.0 | `bindgen` generates deref of null pointers | |
| timsort-0.1.2 | `ptr::copy` invalidation pattern | |
| fftw-sys-0.6.0 | `bindgen` generates deref of null pointers | |
| json_typegen_shared-0.7.0 | `syn` | |
| mucell-0.3.5 | `rulinalg` | |
| rusty-machine-0.5.4 | | |
| stack-graphs-0.4.0 | `bitvec` | |
| cryptographic-message-syntax-0.7.0 | `bytes` | |
| range-map-0.1.5 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| netlify_headers-0.1.1 | `http` | |
| bmrng-0.5.2 | `generator` | |
| mownstr-0.1.3 | Pointer to first element instead of pointer to array | |
| basis-universal-sys-0.1.1 | `bindgen` generates deref of null pointers | |
| bolt-proto-0.11.0 | `bytes` | |
| itm-decode-0.6.1 | `bitvec` | |
| redis-streams-0.1.1 | Old version of `itoa` | |
| sv-parser-0.11.2 | `str-concat` | |
| sv-parser-pp-0.11.2 | `str-concat` | |
| futures_cbor_codec-0.3.1 | `ringbuf` | |
| faiss-sys-0.5.0 | `bindgen` generates deref of null pointers | |
| input-sys-1.16.1 | `bindgen` generates deref of null pointers | |
| http-muncher-0.3.2 | Use of `mem::uninitialized` | |
| endianness-0.2.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| rbpf-0.1.0 | Use of `mem::uninitialized` | |
| tauri-libappindicator-sys-0.1.2 | `bindgen` generates deref of null pointers | |
| nlprule-0.6.4 | `rayon` | |
| ss_ewasm_api-0.11.0 | `wee_alloc` | |
| gvariant-0.4.0 | Out-of-bounds offset | |
| libduckdb-sys-0.3.1 | `bindgen` generates deref of null pointers | |
| dlt-0.9.1 | `nalgebra` | |
| lignin-0.1.0 | `bumpalo` | |
| quickersort-3.0.1 | | |
| protofish-0.5.2 | `bytes` | |
| shakmaty-0.20.4 | `arrayvec` | |
| onnxruntime-sys-0.0.14 | `bindgen` generates deref of null pointers | |
| bulletproof-kzen-1.2.0 | Use of `mem::uninitialized` | |
| linfa-nn-0.5.0 | | |
| kd-tree-0.4.1 | `pdqselect` | |
| sshkeys-0.3.1 | Old version of `generic-array` | |
| bytebuffer-0.2.1 | Misaligned pointer | |
| ssri-7.0.0 | Old version of `generic-array` | |
| gatekeeper-2.2.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| csfml-system-sys-0.6.0 | `bindgen` generates deref of null pointers | |
| cryptoauthlib-sys-0.2.2 | `bindgen` generates deref of null pointers | |
| mpi-sys-0.1.2 | `bindgen` generates deref of null pointers | |
| opc-0.3.0 | Use of `mem::uninitialized` | |
| csfml-window-sys-0.6.0 | `bindgen` generates deref of null pointers | |
| wombo-0.1.2 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| svgparser-0.8.1 | `siphasher` | |
| csfml-graphics-sys-0.6.0 | `bindgen` generates deref of null pointers | |
| kzen-paillier-0.4.2 | Use of `mem::uninitialized | |
| fast_image_resize-0.7.0 | | |
| bit-struct-0.1.31 | Out of bounds offset | |
| prometheus-utils-0.5.1 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| ring-channel-0.9.1 | | |
| unhtml-0.8.0 | `cssparser` | |
| csfml-audio-sys-0.6.0 | `bindgen` generates deref of null pointers | |
| prost-wkt-types-0.3.0 | `bytes` | |
| serde-lexpr-0.1.2 | Old version of `itoa` | |
| sixtyfps-corelib-0.1.6 | | |
| zk-paillier-0.4.2 | `rust-gmp-kzen` | |
| fibers-0.1.13 | `splay_tree` | |
| const-field-offset-0.1.2 | | |
| mbedtls-sys-auto-2.26.1 | `bindgen` generates deref of null pointers | |
| cryptoki-sys-0.1.3 | `bindgen` generates deref of null pointers | |
| edid-0.3.0 | Old `nom` used `mem::uninitialized` | |
| skiplist-0.4.0 | | |
| vtable-0.1.5 | | |
| splay_tree-0.2.10 | | |
| croaring-sys-mw-0.4.5 | `bindgen` generates deref of null pointers | |
| try-lazy-init-0.0.2 | Old version of `crossbeam` | |
| eip55-0.1.1 | `rust-crypto` | |
| cancellation-0.1.0 | |
| sixtyfps-0.1.6 | | |
| libftd2xx-ffi-0.8.4 | `bindgen` generates deref of null pointers | |
| jomini-0.16.4 | `stdarch` | |
| hrpc-0.33.22 | `bytes` | |
| crossbeam_requests-0.3.0 | Old `crossbeam-channel` used `mem::zeroed` incorrectly | |
| ignition-config-0.2.0 | `semver` | |
| html2runes-1.0.1 | Old `tendril` used `mem::uninitialized` | |
| bit-matrix-0.6.1 | | |
| smallset-0.1.1 | | |
| cstree-0.10.0 | `triomphe` | |
| join-0.3.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| ark-gm17-0.3.0 | `rayon` | |
| kolmogorov_smirnov-1.1.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| libsodium-ffi-0.2.2 | `bindgen` generates deref of null pointers | |
| libsensors-sys-0.2.0 | `bindgen` generates deref of null pointers | |
| async-jsonrpc-client-0.3.0 | `bytes` | |
| temporary-0.6.3 | Uses `mem::uninitialized` as a random number generator | |
| usb-disk-probe-0.1.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| rg3d-core-0.19.0 | `nalgebra` | |
| lzf-0.3.1 | Use of `mem::uninitialized` | |
| stream_generator-0.1.0 | `tokio` | |
| sawtooth-zmq-0.8.2-dev5 | `pulse` | |
| gazebo-0.4.4 | | |
| windows_winmd-0.3.1 | | |
| tokio-mockstream-1.1.0 | `futures` | |
| futures-test-preview-0.3.0-alpha.19 | `futures-util` | |
| debruijn-0.3.4 | `stdarch` | |
| moite_moite-0.2.0 | | |
| rayon_croissant-0.2.0 | `rayon` | |
| pagecache-0.19.4 | `crossbeam_epoch` | |
| dasp_slice-0.11.0 | | |
| async-walkdir-0.2.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| aws-sdk-lambda-0.6.0 | `http` | |
| autopilot-0.4.0 | `rgb` | |
| libusb-0.3.0 | `rusb` | |
| monitor-0.1.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| obj-rs-0.7.0 | | |
| pyoxidizer-0.19.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| toml-parse-0.2.11 | `triomphe` | |
| shaku-0.6.1 | Misaligned pointer | |
| tokio-rayon-2.1.0 | `tokio` | |
| tugger-apple-codesign-0.7.0 | `bytes` | |
| gvr-sys-0.7.2 | `bindgen` generates deref of null pointers | |
| nmea-parser-0.9.0 | `bitvec` | |
| oneline-eyre-0.1.0 | Invalid transmute | |
| tugger-rust-toolchain-0.5.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| async-tftp-0.3.5 | `bytes` | |
| cyclors-0.1.2 | `bindgen` generates deref of null pointers | |
| tinysegmenter-0.1.1 | Attempt to construct invalid `char` | |
| ring_buffer-2.0.2 | | |
| unqlite-1.5.0 | Invalid `assume_init` | |
| kuznyechik-0.7.2 | `stdarch` | |
| ipc-queue-0.1.0 | | |
| futures_ringbuf-0.3.1 | | |
| rust-bert-0.17.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| linked-list-0.0.3 | | |
| gray_matter-0.2.1 | `json` | |
| aws-sdk-athena-0.6.0 | `http` | |
| undo-0.47.1 | `arrayvec` | |
| ntex-amqp-codec-0.8.1 | Invalid `assume_init` | |
| wikipedia-0.3.4 | `bytes` | |
| dynamic-array-0.2.3 | Deallocating with the wrong layout | |
| futures-fs-0.0.5 | `futures` | |
| rspec-1.0.0 | `rayon` | |
| raylib-sys-3.7.0 | `bindgen` generates deref of null pointers | |
| leptonica-sys-0.4.1 | `bindgen` generates deref of null pointers | |
| fitsio-sys-0.4.0 | `bindgen` generates deref of null pointers | |
| h3ron-h3-sys-0.13.0 | `bindgen` generates deref of null pointers | |
| runtime-0.3.0-alpha.8 | `futures-util` | |
| strason-0.4.0 | | |
| efd-0.11.1 | `matrixmultiply` | |
| yaxpeax-arm-0.2.3 | `bitvec` | |
| lasy-0.4.1 | `petgraph` | |
| topograph-0.3.1-alpha.1 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| double-checked-cell-async-2.0.2 | Old version of `crossbeam` | |
| bytepack-0.4.1 | Misaligned pointer | |
| simple-server-0.4.0 | Old version of `bytes` | |
| microfft-0.4.0 | `rustfft` | |
| zfp-sys-0.1.10 | `bindgen` generates deref of null pointers | |
| permutator-0.4.3 | | |
| aws-sdk-ecr-0.6.0 | `http` | |
| mqtt4bytes-0.4.0 | `bytes` | |
| ion-c-sys-0.4.13 | `bidgen` generates deref of null pointers | |
| nlprule-build-0.6.4 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| cubeb-core-0.9.0 | | |
| sparkpost-0.5.4 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| timeago-0.3.0 | `siphasher` | |
| nj-sys-3.0.0 | `bindgen` generates deref of null pointers | |
| pool-0.1.4 | | |
| fnm-1.29.2 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| burst-0.0.2 | | |
| libinchi-sys-0.1.0 | `bindgen` generates deref of null pointers | |
| xxhash-c-sys-0.8.1 | `bindgen` generates deref of null pointers | |
| ross-config-2.24.0 | Incorrect `assume_init` | |
| bastion-0.4.5 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| mikktspace-0.2.0 | `nalgebra` | |
| ddoresolver-rs-0.4.2 | `smallvec` | |
| urlshortener-3.0.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| typos-0.8.5 | `stdarch` | |
| nu_plugin_selector-0.43.0 | `tendril` | |
| nu_plugin_xpath-0.43.0 | `sxd-document` | |
| xh-0.15.0 | `http` | |
| djangohashers-1.5.3 | | |
| cacache-10.0.0 | `generic-array` | |
| strcursor-0.2.5 | Out-of-bounds offset | |
| trailer-0.1.2 | Read of padding bytes | |
| aes-stream-0.2.1 | Use of `mem::uninitialized` | |
| may-0.3.20 | `generator` | |
| seshat-2.3.3 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| aws-sdk-ecs-0.6.0 | `http` | |
| erfa-sys-0.1.2 | `bindgen` generates deref of null pointers | |
| libtelnet-rs-2.0.0 | `bytes` | |
| cht-0.5.0 | `crossbeam-epoch` | |
| apint-0.2.0 | | |
| hashconsing-1.5.0 | `rayon` | |
| swiftnav-sys-0.7.0 | `bindgen` generates deref of null pointers | |
| coordgen-0.2.1 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| alloc-from-pool-1.0.4 | `& -> &mut` | |
| sequoia-ipc-0.27.0 | `sha1collisiondetection` | |
| rslint_parser-0.3.1 | `rslint_rowan` | |
| cfn-guard-2.0.4 | `nom_locate` | |
| tor-dirclient-0.0.3 | `bytes` | |
| itermore-0.2.0 | | |
| xoodoo-0.1.0 | `stdarch` | |
| microkelvin-0.10.3 | | |
| dioxus-core-0.1.9 | `bumpalo` | |
| soup-0.5.1 | `html5ever` | |
| snmp-0.2.2 | Use of `mem::uninitialized` | |
| glm-0.2.3 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| dahl-salso-0.6.6 | | |
| mempool-0.3.1 | | |
| crony-0.2.2 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| rslint_rowan-0.10.0 | int-to-pointer cast | |
| a2-0.6.2 | `bytes` | |
| wg-0.2.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| aws-sdk-kms-0.6.0 | `http` | |
| cogo-0.1.30 | `generator` | |
| biosphere-0.2.1 | `ndarray` | |
| ntex-redis-0.3.1 | Incorrect `assume_init` | |
| gbm-sys-0.2.0 | `bindgen` generates deref of null pointers | |
| gotham_middleware_diesel-0.4.0 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| dimensioned-0.7.0 | `generic-array` | |
| emacs_module-0.18.0 | `bindgen` generates deref of null pointers | |
| libinjection-0.2.4 | `bindgen` generates deref of null pointers | |
| tower-web-0.3.7 | `bytes` | |
| pdf-writer-0.4.1 | Old version of `itoa` | |
| simple_moving_average-0.1.2 | `rayon` | |
| ic-kit-0.4.3 | | |
| staticvec-0.11.2 | `copy` invalidation| |
| enum-flags-0.1.8 | Constructing an invalid enum | |
| rcu_cell-0.1.10 | | |
| smolscale-0.3.16 | Miri defect | https://github.com/rust-lang/miri/issues/1717 |
| minstant-0.1.1 | Incorrect `assume_init` | |
| rasn-cms-0.5.0 | `bytes` | |
| nshare-0.8.0 | `ndarray` | |
