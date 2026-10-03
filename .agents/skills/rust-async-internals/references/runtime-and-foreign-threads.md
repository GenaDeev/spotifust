# Runtime Setup and Foreign Threads

Reference material for `SKILL.md`. Read it when you write the shared-runtime initializer, set a
thread stack size, or make a thread that tokio does not own start or wait for async work.

## Contents

- [Shared runtime](#shared-runtime)
- [Thread stacks](#thread-stacks)
- [Enter the runtime from a foreign thread](#enter-the-runtime-from-a-foreign-thread)
- [Rules for both shapes](#rules-for-both-shapes)
- [Adopt a received socket](#adopt-a-received-socket)

## Shared runtime

Build one runtime explicitly and share it. Each build creates a new thread pool.

```rust
use std::sync::OnceLock;
use tokio::runtime::{Builder, Runtime};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn runtime() -> std::io::Result<&'static Runtime> {
    if let Some(rt) = RUNTIME.get() {
        return Ok(rt);
    }
    let rt = Builder::new_multi_thread()
        .worker_threads(2) // constrain for the battery and CPU budget
        .thread_name("app-tokio")
        .enable_all()
        .build()?;
    // A racing caller may win; its runtime is kept and `rt` is dropped here.
    // Call this only outside the runtime (a foreign or main thread): the drop
    // of the losing runtime panics inside async code.
    Ok(RUNTIME.get_or_init(|| rt))
}
```

Name the threads. A named thread makes a stack dump readable.

## Thread stacks

`thread_stack_size` sets the stack of every thread the runtime spawns: the workers and the
`spawn_blocking` pool. It does not cover the thread that calls `block_on`, because
`Runtime::block_on` polls the root future on that thread. With no value set, the std default
applies (2 MiB, or `RUST_MIN_STACK`), so a smaller value removes headroom.

## Enter the runtime from a foreign thread

A foreign thread (a JNI call, a C callback, a platform service thread) cannot `.await`. It
enters the runtime through `block_on`. Two shapes exist.

**Shape A: hand off to a worker thread.** The foreign call returns at once. Use this when the
work is long-running and the caller must not block.

```rust
let worker = std::thread::Builder::new()
    .name("app-worker".into())
    // `block_on` polls the root future on this thread; size the stack for it.
    .stack_size(4 * 1024 * 1024)
    .spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime.block_on(run_session(config, fd, cancel, stats))
        }));
        // Record Ok / Err / panic into shared state before the thread exits.
    });
// The foreign thread returns immediately.
// A later stop call cancels the work through the CancellationToken.
```

**Shape B: block the calling thread.** The foreign thread owns the work for its whole lifetime.
Use this only when the caller is a dedicated service thread that has nothing else to do. On a
`multi_thread` runtime, run the work on a worker and block only on its handle:
`rt.block_on(rt.spawn(fut))`. The foreign stack then holds only the `JoinHandle`, and a panic in
`fut` returns as a `JoinError`. `fut` must be `Send + 'static`.

## Rules for both shapes

1. Do not call `block_on` for long-running work on a callback thread that the host expects to
   return promptly.
2. Wrap the `block_on` body in `catch_unwind`. In Shape A, it records a panic into shared state
   before the thread ends. In Shape B, it turns an abort into an error for the host, because
   since Rust 1.81 a panic that reaches an `extern "C"` boundary aborts the process. Both need
   `panic = "unwind"`. The `rust-panic-safety` skill, when it is installed, owns that policy.
3. Take ownership of a received file descriptor with a duplicate, because the host can close the
   original at any time. An `OwnedFd` closes on every error path with no cleanup code.
4. Set a received socket to nonblocking before `from_std` or `AsyncFd::new`. On Unix, `from_std`
   panics in debug builds on a blocking socket. `AsyncFd::new` does not check, and a blocking fd
   then blocks a worker thread inside a read in every build.
5. Reset shared state with an RAII guard, not with cleanup code at each early return. A guard that
   sets the module back to `Idle` on drop keeps the state correct after a panic too.
6. Store the `CancellationToken` in the state-machine variants that own live work (for example
   `Starting` and `Running`), so that stop and destroy paths can always reach it.

## Adopt a received socket

```rust
use std::os::fd::{BorrowedFd, OwnedFd, RawFd};

/// # Safety
/// `raw` must be an open socket for the duration of this call.
unsafe fn adopt_socket(raw: RawFd) -> std::io::Result<tokio::net::TcpStream> {
    // SAFETY: the caller keeps `raw` open for the duration of this call.
    let owned: OwnedFd = unsafe { BorrowedFd::borrow_raw(raw) }.try_clone_to_owned()?;
    let stream = std::net::TcpStream::from(owned);
    stream.set_nonblocking(true)?;
    // `from_std` panics outside a runtime context with I/O enabled.
    tokio::net::TcpStream::from_std(stream)
}
```
