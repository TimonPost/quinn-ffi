<!-- Allow this file to not have a first line heading -->
<!-- markdownlint-disable-file MD041 -->

<!-- inline html -->
<!-- markdownlint-disable-file MD033 -->

<div align="center">

# `Quinn FFI build for C#`

**Provides a thin FFI over [Quinn][Quinn] mainly designed for being used in [DotQuic][DotQuic]**
  
</div>

## Main Components

Rust and C# are both safe languages, however they operate in different paradigms. The FFI is inherently unsafe between the two and therefore the right care has to be taken when designing this in-between layer.  

### Handles
This library tries to minimize insecurity by introducing `Handle<T>`. A `Handle` is a wrapper of a pointer allocated on the heap. This `Handle` is bound by Rust safety rules. Because of those rules the calling application is prevented from abusing rust its rules. This is especially the case of C# were shared write/read access is not uncommon. 

For now, there are two types of Handles: `HandleSync` which accepts only `Send + Sync` and wraps a mutable pointer, and `HandleShared`, which accepts only immutable pointers and automatically implements `Send + Sync` because the pointer cannot be mutated and therefore safe to share between threads. 

```rust 
pub struct HandleSync<'a, T>(*mut T, PhantomData<&'a T>)
where
    T: ?Sized;

pub type ConnectionHandle<'a> = HandleSync<'a, Arc<Mutex<ConnectionImpl>>>;

// Rust
unsafe extern "cdecl" fn read_stream(handle: ConnectionHandle);
// C#
[DllImport]
internal static extern QuinnResult read_stream(ConnectionSafeHandle handle);
read_stream(MyCreatedHandle)
```

A `Handle` is a pointer for the FFI layer, and any pointer given to a particular `external` function ought to be pointing to memory of that type. In C#, `ConnectionSafeHandle` is a [SafeHandle][SafeHandle] which wraps a pointer. A `Safe Handle` in C# and `Handle` in Rust are used as two sides of the same coin. This allows the pointer to be shared from rust to C# and back again.

There are several other types that implement similar semantics: `Ref`, `RefMut` which are respectively an imutable and mutable pointer to a resource allocated by C#. Finally, there is `Out` which points to allocated memory in C# with the intention of initializing it in Rust. This allows us to work with the C# `out` were the called function initializes the calling function its state. 

### [Callbacks][callbacks]

There are several other types that implement similar semantics: `Ref`, `RefMut` which are respectively an immutable and mutable pointer to a resource allocated by C#. Finally, there is `Out` which points to allocated memory in C# with the intention of initializing it in Rust. This allows us to work with the C# `out` were the called function initializes the calling function its state. 

Invoking Rust with C# comes at some cost due to `PInvoke` function. It is seen as a good practice to reduce C# => Rust calls as much as possible. Since events occur once in a while this library allows to set callbacks that are called when events trigger.  See the [docs][callbacks] for what function interface the callback methods have to adhere to. 

The client application `MUST` provide a callback for each function before the application starts running. [DotQuic][DotQuic] implements events for the given callbacks and enables different listeners for those events. And these listeners can in turn perform API actions. Be careful about calling FFI within the event handlers, as this can result in deadlocks since the callbacks are invoked in rust that probably locks some handle. 

### Functions
Furthermore those are the [functions][functions] that are supported now: 
```
accept_stream
connect_client
create_client_endpoint
create_server_endpoint
handle_datagram	
last_error
open_stream	
poll_connection	
read_stream
write_stream
```

### Safety

This may change in the future if it is not deemed useful. There are two api's (enabled by feature flag): 
- `safe-api`, performs null checks on each passed pointer to Rust, and catches all panics. 
- `unsafe-api`, does not perform null checks on any pointer to Rust, and does not catch panics. 


## Contribution


### License

This contribution is dual licensed under EITHER OF

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

[callbacks]: /ffi/bindings/callbacks/index.html
[functions]: /ffi/bindings/index.html
[Quinn]: https://github.com/quinn-rs/quinn
[QUIC]: https://en.wikipedia.org/wiki/QUIC
[DotQuic]: https://github.com/TimonPost/dot-sharp
[SafeHandle]: https://docs.microsoft.com/en-us/dotnet/api/system.runtime.interopservices.safehandle?view=net-6.0