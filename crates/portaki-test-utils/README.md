# portaki-test-utils

In-process **mock host** for Portaki module unit tests.

Runs renderer / query / command logic **without** Wasm or Extism. You build a `MockContext`, optionally stub connector responses and KV/repo state, then assert on the returned `Surface` tree.

## Install

```toml
[dev-dependencies]
portaki-sdk = "0.1"
portaki-test-utils = "0.1"
```

## Example

```rust,ignore
use portaki_sdk::prelude::*;
use portaki_test_utils::{MockContext, SurfaceAssertions};

#[test]
fn guest_home_renders_title() {
    let ctx = MockContext::guest().build();
    let surface = guest_home(&ctx).expect("render");
    SurfaceAssertions::new(&surface)
        .assert_contains(/* primitive tag or text */);
}
```

API reference: [docs.rs/portaki-test-utils](https://docs.rs/portaki-test-utils).

## License

Apache-2.0
