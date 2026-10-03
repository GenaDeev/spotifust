# Cancellation and Shutdown

Reference material for `SKILL.md`. Read the section that matches the code you write or review.

## Contents

- [Cancel safety is an untyped invariant](#cancel-safety-is-an-untyped-invariant)
- [Library method cancel-safety table](#library-method-cancel-safety-table)
- [Spawn-and-join firewall for critical sections](#spawn-and-join-firewall-for-critical-sections)
- [Disabled select! branches](#disabled-select-branches)
- [CancellationToken idioms](#cancellationtoken-idioms)
- [Task sets and supervision](#task-sets-and-supervision)
- [Cooperative cancellation of CPU work](#cooperative-cancellation-of-cpu-work)
- [block_in_place starves join! branches](#block_in_place-starves-join-branches)
- [broadcast receivers and Lagged](#broadcast-receivers-and-lagged)
- [Async-Drop contracts of pooled resource libraries](#async-drop-contracts-of-pooled-resource-libraries)

## Cancel safety is an untyped invariant

A future is **cancel safe** if and only if dropping it between any two `.await` points leaves
observable state consistent. No signature expresses the property. `async fn f(...)` looks the
same whether `f` is safe to cancel between its internal `.await`s or not. The borrow checker and
Clippy do not help. The information lives in the caller context (whether the future ends up in
`tokio::select!`, `tokio::time::timeout`, or `FuturesUnordered`) plus the library documentation
of each method.

By default, annotate each `async fn` that can be polled, directly or transitively, inside
`select!`, `timeout`, or `FuturesUnordered` with a doc comment of this form, because no
signature, lint, or test tool records cancel safety:

```rust
use tokio::net::TcpStream;

struct Request;
struct Connection;

impl Connection {
    /// cancel-safe: only `.await`s on `read` and `mpsc::recv`, both individually cancel-safe.
    async fn read_request(&mut self) -> Result<Request> { todo!() }

    /// NOT cancel-safe: `db.insert().await` followed by `send_ack().await` —
    /// cancellation between them leaves the DB written but the client unacked.
    async fn process(&self, stream: TcpStream) -> Result<()> { todo!() }
}
```

Prefix the comment with `cancel-safe:` or `NOT cancel-safe:` and give a reason. "cancel-safe
because idempotent" is not a reason. Idempotence is a property of the operation; cancel safety
is a property of the scheduling. Both must hold independently.

## Library method cancel-safety table

The tokio rows come from the `select!` documentation of tokio 1.53.1.

| Method | Cancel safe? | Why |
|---|---|---|
| `AsyncReadExt::read`, `read_buf` | Yes | On cancellation, no bytes are consumed. |
| `AsyncReadExt::read_exact`, `read_to_end`, `read_to_string` | **No** | Bytes read before cancellation are lost to the caller. |
| `AsyncWriteExt::write`, `write_buf` | Yes | A single write. |
| `AsyncWriteExt::write_all` | **No** | Same partial-write hazard. |
| `mpsc::Receiver::recv`, `broadcast::Receiver::recv`, `watch::Receiver::changed` | Yes | Documented cancel safe. |
| `TcpListener::accept`, `UnixListener::accept` | Yes | Documented cancel safe. |
| `StreamExt::next` (tokio-stream, futures) | Yes | Documented cancel safe. |
| awaiting `&mut oneshot::Receiver` | Yes | Await `&mut rx` in a loop; `rx` by value moves the receiver into the first iteration. |
| `tokio::time::sleep` | Yes | Cancellation drops the timer. |
| `Mutex::lock`, `RwLock::read`, `RwLock::write`, `Semaphore::acquire` | **No (queue position)** | They queue for fairness. Cancellation removes the waiter, so a retry loses its place. No lock or permit leaks. |
| `Notify::notified` | **No (queue position)** | Same queue rule. To avoid a missed `notify_one`, create the future before you check the condition, pin it, call `Notified::enable()`, check, then await. |
| `sqlx::Transaction::commit` | **No** | A drop before success queues a rollback; nothing reports the failure. |
| `sqlx` query futures (`fetch_one` and similar) | **No** for the server statement | Dropping the future does not cancel the statement on the server. In sqlx 0.9.0 the drop of a `PoolConnection` spawns a release task that pings the connection. On Postgres the ping waits until the abandoned statement finishes, and only then does the connection, with its pool slot, return to the pool. Abandoned long queries can exhaust the pool. On a connection you own, the next query waits instead. MySQL and SQLite were not checked. |
| `reqwest::RequestBuilder::send` | **No** (outcome unknown) | After the request starts, the server may have acted. reqwest documents no cancel-safety guarantee. The `rust-networking` skill, when it is installed, owns cancellation at the network boundary. |

## Spawn-and-join firewall for critical sections

When a sequence of `.await`s must complete atomically with respect to cancellation, lift it
into a spawned task and join the handle:

```rust
async fn process(stream: TcpStream, db: Arc<Db>) -> Result<()> {
    let data = read_message(&stream).await?;
    // From here on, cancellation of `process()` must NOT abort the work.
    let handle = tokio::spawn(async move {
        db.insert(&data).await?;
        send_ack(&stream).await?;
        Ok::<_, Error>(())
    });
    handle.await?  // outer cancellation cancels the join, not the spawned work.
}
```

This trades cooperative cancellation for atomicity. The spawned task runs to completion even if
the caller is dropped. Use it only when the alternative (data loss or inconsistent state) is
worse. Put a `tokio::time::timeout` inside the spawned task if unbounded run time is itself a
hazard.

## Disabled select! branches

Tokio evaluates every branch precondition first. It then evaluates every async expression,
including the expressions of disabled branches. It does not poll a disabled future. This matters
when the construction of the future does synchronous work:

```rust
let branch = false;
tokio::select! {
    _ = async {
        // Put side effects here. This body runs only when the future is polled.
        prepare_then_wait().await;
    }, if branch => {}
    _ = ready() => {}
}
```

Do not write `make_future_with_side_effect()` as the disabled branch expression. Its call runs
before Tokio decides which futures to poll.

## CancellationToken idioms

`tokio_util::sync::CancellationToken` has two idioms beyond `.cancelled().await` and the child
tokens that `SKILL.md` shows.

**DropGuard: cancellation on an RAII boundary.**

```rust
use tokio_util::sync::CancellationToken;

let token = CancellationToken::new();
let _guard = token.clone().drop_guard();
// ... do work, possibly with early returns / ? ...
// When _guard drops (early return, panic, end of scope), the token is
// cancelled automatically. Spawned tasks that hold `token.clone()` observe it.
```

Use it for a bounded async operation where cancellation must fire on any early exit. It replaces
hand-written cleanup at each `?`. `drop_guard_ref()` (tokio-util 0.7.16+) borrows the token
instead of consuming it.

**`run_until_cancelled`: race a future against cancellation and keep its value.**

```rust
use tokio_util::sync::CancellationToken;

let token = parent.child_token();
match token.run_until_cancelled(do_work()).await {
    Some(value) => process(value),              // do_work completed
    None => tracing::info!("cancelled"),        // cancellation fired first
}
```

`SKILL.md`, section "CancellationToken tree", gives its semantics on an already-cancelled token.

## Task sets and supervision

tokio has no scoped-task API. Build structured shutdown from a `CancellationToken` plus a
`JoinSet` or a `TaskTracker`. `SKILL.md`, section "Task ownership and concurrency", gives the
`JoinSet`, `TaskTracker`, and `FuturesUnordered` rules. This accept loop is the extended
`TaskTracker` example:

```rust
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

async fn serve(listener: tokio::net::TcpListener, cancel: CancellationToken) {
    let tracker = TaskTracker::new();
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            accepted = listener.accept() => {
                let socket = match accepted {
                    Ok((socket, _peer)) => socket,
                    Err(e) => {
                        // EMFILE and similar errors return at once: back off, do not spin.
                        tracing::warn!(%e, "accept failed");
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };
                let child = cancel.child_token();
                tracker.spawn(async move {
                    let _ = child.run_until_cancelled(handle(socket)).await;
                });
            }
        }
    }
    tracker.close();
    tracker.wait().await; // every connection task has exited
}

async fn handle(_socket: tokio::net::TcpStream) {}
```

Supervision (restart on failure): no tokio-maintained supervisor exists. Write an explicit
restart loop with a bounded backoff and a `CancellationToken` check, and document the restart
policy next to it.

## Cooperative cancellation of CPU work

`timeout` around `spawn_blocking` bounds only the caller's wait. A started blocking closure runs
to its end unless it checks a token:

```rust
use std::time::Duration;
use tokio_util::sync::CancellationToken;

fn expensive_cpu_computation(cancel: &CancellationToken) {
    while !cancel.is_cancelled() {
        if do_one_bounded_work_unit() {
            break;
        }
    }
}

fn do_one_bounded_work_unit() -> bool { todo!() }

let cancel = CancellationToken::new();
let worker_cancel = cancel.clone();
let result = tokio::time::timeout(
    Duration::from_secs(1),
    tokio::task::spawn_blocking(move || expensive_cpu_computation(&worker_cancel)),
).await;
if result.is_err() {
    cancel.cancel();
}
```

Apply this when a caller wraps CPU work in `timeout` or must stop it at shutdown. Choose a work-unit size that meets the written cancellation-latency budget. Do not describe
the operation as time-bounded unless the blocking function checks the token. The same token
check is the only way to stop the closure at shutdown; the `SKILL.md` triage table gives the
symptom.

## block_in_place starves join! branches

`tokio::task::block_in_place` moves the current worker thread to the blocking pool and gives
its other tasks to the remaining workers. Inside a `join!`, the other branches run on the same
task, so `block_in_place` suspends them for the whole blocking call. Branch A completes, and
branch B runs only afterwards. Use `spawn_blocking` instead. It works on every runtime flavor and
does not affect co-located futures.

## broadcast receivers and Lagged

`tokio::sync::broadcast` has a fixed ring-buffer capacity. A slow receiver that falls behind has
its oldest messages overwritten, and its next `recv()` returns `Err(RecvError::Lagged(n))`.

```rust
use tokio::sync::broadcast::{self, error::RecvError};

fn process(_msg: u32) {}

// BUG: the first Lagged ends the loop. The consumer stops for good and logs nothing.
async fn consume_until_first_error(mut rx: broadcast::Receiver<u32>) {
    while let Ok(msg) = rx.recv().await {
        process(msg);
    }
}
```

```rust
use tokio::sync::broadcast::{self, error::RecvError};

fn process(_msg: u32) {}

async fn consume(mut rx: broadcast::Receiver<u32>) {
    loop {
        match rx.recv().await {
            Ok(msg) => process(msg),
            Err(RecvError::Lagged(n)) => {
                tracing::warn!(dropped = n, "broadcast receiver lagged");
                // Decide: continue, alert, or resynchronize from a snapshot.
            }
            Err(RecvError::Closed) => break,
        }
    }
}
```

For audit logs, metrics, or state-machine transition messages, a `Lagged` drop is data loss.
Use `mpsc` with explicit backpressure for lossless delivery.

## Async-Drop contracts of pooled resource libraries

Async types whose `Drop` performs cleanup (transactions, connections, file handles) have
library-specific behavior that their signatures do not show. Read the `Drop` impl before you
rely on it.

### sqlx transactions

Read against sqlx 0.7.4, 0.8.6, and 0.9.0. The `Drop` body is the same in all three. The
`rust-database` skill, when it is installed, owns the minimum sqlx version, commit, rollback, and
unknown-commit-outcome handling.

```rust
let tx = conn.begin().await?;
// ... operations ...
tx.commit().await?;  // Consumes `tx`; an error can leave the durable outcome unknown.
```

`Transaction::drop` calls `TransactionManager::start_rollback`, a non-async function that queues
a ROLLBACK on the connection. Nothing blocks in `drop`. The rollback runs on the next async use
of that connection, which includes the moment the pool recycles it. This cleans up a transaction
that the driver still considers open. It does not prove that a failed `commit` was absent from
the database. For example, the database can commit and the connection can fail before the
acknowledgement reaches the client.

### deadpool connections

Read against deadpool 0.13.0 and 0.13.1. `Object::drop` pushes the connection back into the pool
slot list and adds a permit. It is fully synchronous and runs no health check.
`Manager::recycle`, the `pre_recycle` hook, and the `post_recycle` hook all run inside the next
`Pool::get().await`, not at drop time.

Consequence: a connection returned by `Drop` during shutdown keeps whatever state the last query
left on it. Nothing validates it, and if the process exits before another `get()`, `recycle`
never runs at all.

In shutdown paths, take the object out of the pool with `Object::take(obj)`, which detaches it,
and close it yourself. Do not expect `Drop` to run any hook.

### tokio::fs::File

Read against tokio 1.53.1. `File` has no `Drop` impl at all. It holds `Arc<std::fs::File>`, so
the fd closes when the last handle to that `Arc` drops. A write started by `poll_write` runs on
the blocking pool and holds a clone until it ends, so the close waits for it. With no operation
in flight, the close syscall runs inline, on the thread that drops the file.

The data is not lost, but the write error is. A failed background write lands in
`last_write_err`, and the next `write`, `flush`, or `seek` call reads that field. A drop makes
that call never happen.

Call `file.flush().await?` before you drop a file you wrote to. Add `file.sync_all().await?` when
the bytes must reach the disk.

### General audit

For every `Drop` on an async resource type:

1. Read the library source for `impl Drop`. Does it block, spawn, queue the work for a later
   call, or do nothing? A missing `Drop` impl is also an answer.
2. If it blocks, run cleanup through an explicit `.commit()`, `.rollback()`, or `.close()` before
   the drop.
3. If it spawns or queues, cleanup is fire-and-forget and reports no error. Verify the behavior
   under runtime shutdown.
4. Document the choice in a comment on the variable binding, for example
   `// drop here: deferred rollback acceptable in error path`.
