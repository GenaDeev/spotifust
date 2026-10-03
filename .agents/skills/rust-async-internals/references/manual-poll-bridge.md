# Manual Poll Bridge

Reference material for `SKILL.md`. Read it when you drive tokio streams from a synchronous,
step-driven loop, or when you drive an io_uring backend from that loop.

## The pattern

The bridge is a manual poll with the standard no-op waker, `Waker::noop()` (Rust 1.85). It
allocates nothing. Clippy's `manual_noop_waker` lint flags a hand-written replacement.

```rust
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use tokio::io::{AsyncRead, ReadBuf};

/// Call only from the synchronous loop tick, never from async code.
fn try_read_duplex(
    stream: &mut tokio::io::DuplexStream,
    buf: &mut [u8],
) -> Option<io::Result<usize>> {
    let mut cx = Context::from_waker(Waker::noop());
    let mut rb = ReadBuf::new(buf);
    match Pin::new(stream).poll_read(&mut cx, &mut rb) {
        // With a non-empty `buf`, a length of 0 is EOF.
        Poll::Ready(Ok(())) => Some(Ok(rb.filled().len())),
        Poll::Ready(Err(e)) => Some(Err(e)),
        Poll::Pending => None, // the loop retries on the next tick
    }
}
```

## Why the waker does nothing

The `try_*` functions run inside the synchronous loop tick, not inside a tokio task. No task
exists that a wake could reschedule, so a real waker would be wasted work.

The trade-off is that `Poll::Pending` carries no progress information. The loop decides when to
poll again, from its own timing, not from tokio wake signals.

## Invariants

1. **Do not call a no-op-waker poll helper from inside an async task, and do not wrap it in an
   `async fn`.** Under a no-op waker, `Poll::Pending` means that no wake will ever arrive. The
   task stalls for ever. Call these helpers only from synchronous loop code.
2. **`Pin::new` needs `Unpin`.** `Pin::new(&mut stream)` is sound here because
   `DuplexStream: Unpin`. A stream type that is not `Unpin` must be pinned once, for example in
   a `Pin<Box<_>>` field, and polled through `as_mut()`.
3. **Cancellation is cooperative, not waker-driven.** The loop checks a `CancellationToken`
   between polling rounds.

## Extend the bridge

- A new stream type implements `AsyncRead + AsyncWrite + Unpin`.
- `poll_read` returns `Poll<Result<()>>`, not a byte count. A read wrapper records
  `ReadBuf::filled().len()` before the poll. On `Poll::Ready(Ok(()))`, no increase means EOF.
  Translate EOF to `UnexpectedEof` only when the protocol requires more bytes.
- `poll_write` returns a byte count. A write wrapper translates `Poll::Ready(Ok(0))` into
  `WriteZero`, and handles `BrokenPipe` as a closed peer.

## io_uring registered buffers

When the loop drives an io_uring backend with registered buffer pools for zero-copy `SendZc` and
fixed-buffer `ReadFixed` (`IORING_OP_READ_FIXED`), hold these invariants:

1. Register buffers once at startup, and reference them by index in every SQE. A second
   `IORING_REGISTER_BUFFERS` fails with `EBUSY` while buffers are registered. Replace a slot with
   `IORING_REGISTER_BUFFERS_UPDATE` (Linux 5.13+); the kernel can keep the old buffer alive
   until the requests that use it complete. Before Linux 5.13, registration waits for the ring
   to go idle.
2. Do not reuse a buffer index until the CQE of every request that uses it has arrived.
3. tokio does not synchronize SQE submission and CQE completion. The loop polls the ring and
   drains completions into per-consumer queues.
4. Every `unsafe` block that constructs an SQE carries a `// SAFETY:` comment that names the
   buffer-index validity and the fd lifetime. The `rust-unsafe` skill, when it is installed, owns
   the convention.
5. Cancel an in-flight operation with `IORING_OP_ASYNC_CANCEL`; do not just drop it. After the
   cancel, keep the buffer and the fd owned until the target request posts its CQE. A cancel
   result of `-EALREADY` means that the request is still running and will complete. Reuse before
   the target CQE lets the kernel write into memory that Rust considers free.
