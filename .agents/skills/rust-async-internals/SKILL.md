---
name: rust-async-internals
description: Use when writing, reviewing, or debugging tokio task and future mechanics, including cancel safety in tokio::select!, timeout, JoinSet, or FuturesUnordered; task ownership and shutdown with JoinHandle, TaskTracker, or CancellationToken; blocking work via spawn_blocking, block_in_place, or block_on from a foreign thread; runtime setup; manual polling from a synchronous loop with Waker::noop; and Send bounds on an async closure, AsyncFn, or async fn in traits. Triggers on "disabled select branch", "shutdown hang", "async hang", "task stall", a !Send future, broadcast Lagged, and a MutexGuard held across .await.
license: BSD-3-Clause
---

# Rust Async Internals

## Decision table

| You must | Use | Do not use |
|---|---|---|
| Run N futures and keep only the first result | `tokio::select!` | `join!` |
| Run N futures to completion | `tokio::join!` or `try_join!` | `select!` |
| Own a set of spawned tasks and collect their results | `JoinSet` + `join_next` | a bare `Vec<JoinHandle>` |
| Track a long-lived, unbounded task set (an accept loop) for shutdown | `tokio_util::task::TaskTracker` + `close()` + `wait()` | a `JoinSet` that nobody drains; it keeps every result |
| Abort a task when its owner drops the handle | `tokio_util::task::AbortOnDropHandle` | a bare `JoinHandle`; its drop detaches the task |
| Bound the concurrency of a work stream | `stream::iter(..).buffer_unordered(K)` | `spawn` in an unbounded loop |
| Shut down a task tree | `CancellationToken` + `child_token()` | `Notify` |
| Cancel on any early return or panic | `token.drop_guard()` | manual cleanup at each `?` |
| Race work against shutdown and keep the value | `token.run_until_cancelled(fut)` | a hand-written `select!` |
| Do bounded blocking or CPU work | `spawn_blocking` | `block_in_place` |
| Run an indefinite blocking loop | `std::thread::spawn` | `spawn_blocking` |
| Hold a lock across `.await` | `tokio::sync::Mutex` | `std::sync::Mutex` |
| Deliver every message without loss | `mpsc` | `broadcast` |
| Bound how long a caller waits for CPU work | `timeout(d, spawn_blocking(..))` | `timeout(d, cpu_work())` |
| Keep a critical `.await` sequence atomic | `tokio::spawn` + join the handle | `select!` around it |

## Stall and hang triage

| Symptom | Likely cause | Check or fix |
|---|---|---|
| Every task is slow, one task looks stuck | a blocking call inside async context | grep for `std::thread::sleep`, `std::fs`, `std::net`, synchronous HTTP or DB calls; move them as the [blocking table](#blocking-work) says |
| Shutdown never completes | a `select!` loop without a `cancelled()` arm | add `biased;` + `_ = cancel.cancelled() => break` |
| Async tasks abort, but the process does not exit | a started `spawn_blocking` closure; `abort`, `JoinSet` drop, and runtime shutdown cannot stop it | pass a `CancellationToken` into the closure and check it; `Runtime::shutdown_timeout` stops only the wait |
| Latency spikes with no obvious cause | long-lived tasks saturate the blocking pool | move indefinite work to `std::thread::spawn` |
| `spawn_blocking` work hangs on tokio 1.52.0 | a regression in exactly 1.52.0 (tokio issue #8056) | `cargo update -p tokio` to 1.52.1 or later |
| A timeout is exceeded but returns `Ok`, or never returns | one poll of the wrapped future does not yield | wrap `spawn_blocking` inside the `timeout`; add cooperative cancellation if the work itself must stop |
| Panic: "can call blocking only when running on the multi-threaded runtime" | `block_in_place` on a `current_thread` runtime, including a default `#[tokio::test]` | use `spawn_blocking` |
| Panic: "Cannot start a runtime from within a runtime" | `block_on` on a thread that already drives async tasks | `.await` the future; run synchronous code that must block on a `std::thread` or in `spawn_blocking` with a `Handle` |
| Events are missing, no error is logged | a `broadcast` receiver lagged; `while let Ok(..)` ends the loop at the first `Lagged` | match `RecvError::Lagged(n)` explicitly, or switch to `mpsc` |
| Deadlock only under concurrent load | a `std::sync::Mutex` guard held across `.await` | run `clippy::await_holding_lock`; drop the guard before the `.await`, or use `tokio::sync::Mutex` |
| A stream stalls forever after one `Pending` | a no-op-waker poll helper was called from async code | move the call into the synchronous loop tick |
| Writes succeed, the peer sees nothing | the synchronous engine did not run a step after the write | check that the loop does not skip ticks under load |
| io_uring operations hang or corrupt memory after cancellation | an SQE was dropped without `IORING_OP_ASYNC_CANCEL`, or its buffer was reused before the target CQE | submit the cancel, and keep the buffer and fd alive until the target's CQE arrives |
| File descriptors accumulate | a raw fd without an owner on an error path | hold every fd as `OwnedFd` so each early return closes it |

## Verify

| Claim | Check | A green result does not prove |
|---|---|---|
| No lock guard or `RefCell` borrow crosses `.await` | `cargo clippy --locked --all-targets -- -D clippy::await_holding_lock -D clippy::await_holding_refcell_ref` | cancel safety, or lock order between tasks |
| Shutdown is bounded | a test that keeps one task busy, cancels the token, and asserts `tokio::time::timeout(deadline, handle).await.is_ok()`; run it with `flavor = "multi_thread"` too | shutdown under production load |
| Timeout and retry logic | `#[tokio::test(start_paused = true)]` (tokio `test-util` feature) with `tokio::time::advance` | a blocking poll; paused time does not bound CPU work |
| Code that calls `block_in_place` works | `#[tokio::test(flavor = "multi_thread")]` | the default `#[tokio::test]` flavor, which panics |
| No known advisory (vulnerability or unsound) for tokio or tokio-util | `cargo deny --config deny.toml --locked check advisories` with `unsound = "all"`, or `cargo audit --deny warnings` | behavior regressions that have no advisory |

No compiler, lint, or test tool checks cancel safety. Review the `cancel-safe:` annotations, and
test a cancel-sensitive future by dropping it at each `.await` (for example, race it in `select!`
against a `oneshot` that fires at a chosen step). Then assert the state invariant.

Plain `cargo audit` exits 0 on an `unsound` advisory, and the cargo-deny default
`unsound = "workspace"` misses a transitive tokio. Show the resolved version with
`cargo tree --locked -i tokio`. Do not keep a hand-written minimum-version table: one floor
misses backport lines. For example, the unsound advisory RUSTSEC-2025-0023 (the broadcast channel
calls `clone` on a `Send + !Sync` value from several threads) is patched in `>=1.38.2,<1.39`,
`>=1.42.1,<1.43`, `>=1.43.1,<1.44`, and `>=1.44.2`. A floor of 1.42.1 accepts the affected
1.43.0, 1.44.0, and 1.44.1. The `rust-security` skill, when it is installed, owns that policy.

## Completion criteria

Check these before you call async work done, and when you review async code or approve a merge.

- [ ] By default, every future in a `select!`, `timeout`, or `FuturesUnordered` arm is
      annotated `cancel-safe:` or `NOT cancel-safe:` with a reason.
- [ ] Every long-lived `select!` loop has a `cancelled()` arm, and uses `biased;` when shutdown
      must win.
- [ ] A disabled `select!` branch has no synchronous side effect in its async expression.
- [ ] Every spawned task has an owner that cancels or aborts it and then joins it.
- [ ] No blocking syscall or CPU-heavy loop runs on a runtime worker thread.
- [ ] `spawn_blocking` holds only bounded work; indefinite work uses `std::thread::spawn`.
- [ ] Every `broadcast` receive loop handles `RecvError::Lagged`.
- [ ] Every received file descriptor is an `OwnedFd`, and every received socket is nonblocking.
- [ ] No `for .. { tokio::spawn(..) }` loop lacks a `JoinSet`, a `TaskTracker`, or a concurrency
      bound.
- [ ] No async wrapper exists around a no-op-waker poll helper.

## Cancellation

`select!` completes when the first branch finishes. It drops every other branch future at that
instant, and the work in progress inside a dropped branch is lost. `join!` waits for all
branches and has no cancellation surprise.

Every future in a `select!` arm, in `timeout`, or in `FuturesUnordered` must therefore be cancel
safe. Read [references/cancellation-and-shutdown.md](references/cancellation-and-shutdown.md)
when you annotate cancel safety, pick a library method for a `select!` arm, or protect a
sequence of `.await`s that must not be split.

A disabled branch still evaluates its async expression. Tokio does not poll the resulting future,
but synchronous setup in the expression can allocate, lock, mutate state, or panic. Move side
effects into the async body, or compute the branch only after its precondition.

### Put shutdown first with `biased`

```rust
loop {
    tokio::select! {
        biased;
        _ = cancel.cancelled() => break,  // always polled first
        msg = rx.recv() => process(msg),
    }
}
```

Without `biased`, `select!` picks a random arm to poll first, so the loop can still process a few
more messages after cancellation fires. `biased;` with the `cancelled()` arm first makes shutdown
win on the next iteration. Under `biased;`, never put an always-ready data arm above the shutdown
arm: that order starves it. Give every long-lived `select!` loop a `cancelled()` arm unless
another arm ends the loop on shutdown. Otherwise shutdown hangs.

### CancellationToken tree

Use `tokio_util::sync::CancellationToken` for structured shutdown, not `tokio::sync::Notify`.

```rust
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

async fn run_job(_job: u32, _cancel: CancellationToken) {}

async fn run_all(jobs: Vec<u32>) {
    let master = CancellationToken::new();
    let tracker = TaskTracker::new();
    for job in jobs {
        tracker.spawn(run_job(job, master.child_token()));
    }
    // On shutdown: a child's cancel() stops only that child; master.cancel() stops all.
    master.cancel();
    tracker.close();
    tracker.wait().await;
}
```

`run_until_cancelled(fut)` returns `None` and never polls `fut` when the token is already
cancelled (tokio-util 0.7.16+). After that it polls `fut` first on each wake. The losing `fut`
is still dropped, so it must be cancel safe.

## Task ownership and concurrency

- `JoinSet` owns a set of spawned tasks. `join_next().await` yields the next completed result.
  Dropping the `JoinSet` aborts the tasks but does not wait for them. It keeps each result until
  you call `join_next`, so an undrained `JoinSet` in an accept loop grows without limit.
- `TaskTracker` counts tasks and keeps no results. Call `close()`, then `wait().await`, on
  shutdown. Dropping it does not abort the tasks, so pair it with a `CancellationToken`.
- `FuturesUnordered` polls a set of futures in place, without spawning, and drops a future that
  you remove from it.
- `buffer_unordered(K)` bounds concurrency to K. Its in-flight futures make progress only while
  the stream is polled. A slow `.await` in the consuming loop body stalls them and can fire
  their timeouts. Keep the loop body short, or spawn the work.

Dropping a bare `JoinHandle` detaches its task. It does not cancel the task, and the result or
panic is lost. Keep the handle, signal cooperative cancellation, and await it. Use `abort()` only
when abrupt cancellation is part of the task contract, and still await the handle to observe
the exit.

A `for x in xs { tokio::spawn(work(x)); }` loop with N > 1 is a refactor candidate. Replace it
with `JoinSet::spawn` + `join_next`, with `futures::future::join_all` for a small fixed N, with
`buffer_unordered(K)` for a stream, or with `TaskTracker` for an unbounded set.

Give every long-lived task its own `tracing` span, so that a stalled task is visible in the log
by name.

## Blocking work

| Work shape | Mechanism | Reason |
|---|---|---|
| Bounded CPU work, target < 100 ms | `spawn_blocking` | it returns the pool thread quickly |
| Occasional blocking syscall, short file I/O | `spawn_blocking` | same |
| Many CPU-bound jobs at once | a `Semaphore` around `spawn_blocking`, or `rayon` | the blocking pool admits up to 512 threads by default |
| Data-parallel compute (decode, geometry, raster) | `rayon` inside one `spawn_blocking`, or `rayon::spawn` plus a `oneshot` for the result | a `rayon` call blocks its caller until the parallel work ends |
| Indefinite blocking loop, watcher, persistent synchronous connection | `std::thread::spawn` | it would occupy a pool thread for ever |
| Anything on a `current_thread` runtime | `spawn_blocking` | `block_in_place` panics there |
| Blocking I/O in a synchronous engine with no runtime | keep it synchronous | a runtime adds cost and no benefit there |

Load data before a `rayon` region; do not do blocking I/O inside a `rayon` task.

A synchronous protocol client that runs on its own `std::thread`, outside the runtime, is a valid
design. Write a comment at the spawn site that states the intent, so that a later reader does
not "fix" it into async.

## Send, 'static, and async bounds

`tokio::spawn` requires `Send + 'static`. A value that lives across an `.await` becomes part of
the future, so one `!Send` value makes the whole future `!Send`.

- A `std::sync::MutexGuard` or a `RefCell` borrow held across `.await` compiles wherever the
  future need not be `Send`: the root future of `block_on` or `#[tokio::main]`, and
  `spawn_local` tasks. Under concurrent load it deadlocks or panics.
  `clippy::await_holding_lock` and `clippy::await_holding_refcell_ref` find both; they warn by
  default.
- Fix by shortening the scope. Copy what you need, drop the guard, then `.await`. Use
  `tokio::sync::Mutex` only when the lock must be held across the `.await`.
- Move owned values or `Arc<T>` handles into a spawned task. A captured `&T` fails the
  `'static` bound.

Pick the callback or trait shape from where the future goes:

| Need | Shape | Reason |
|---|---|---|
| Callback over `&T`; the future stays on the caller's task (Rust 1.85+) | `F: AsyncFn(&T) -> R` | no box; the borrow works |
| Callback whose future crosses `tokio::spawn` or a `dyn` boundary | `F: for<'a> Fn(&'a T) -> Pin<Box<dyn Future<Output = R> + Send + 'a>>` | no stable bound names the `AsyncFn` future, so `F: AsyncFn(..) + Send` does not make it `Send` |
| Trait method that callers spawn through a generic `T: Trait` | `fn m(&self) -> impl Future<Output = R> + Send`, or `#[trait_variant::make(TraitSend: Send)]` | a native `async fn` in a trait adds no `Send` bound |
| Trait used as `dyn Trait` | `#[async_trait]`, a method that returns `Pin<Box<dyn Future<Output = R> + Send + '_>>`, or the `dynosaur` crate | `async fn` and `-> impl Future` methods are not dyn compatible (E0038); `trait_variant` does not change that |

A native `async fn` call on a concrete type still spawns, because the opaque type leaks its auto
traits. The same call through a generic `T: Trait` bound fails with
`error: future cannot be sent between threads safely`, which has no error code. Do not emit
return type notation (`T::m(..): Send`): it is nightly-only on Rust 1.98.1 and fails with E0658
on stable. Read [references/async-bounds.md](references/async-bounds.md) when a bound fails to
compile, before you migrate off `#[async_trait]`, when edition 2024 changes what an
`impl Future` return captures, or when concurrent futures need one `&mut State`.

## Timeouts

`tokio::time::timeout` polls the wrapped future before it checks the deadline. It cannot preempt
one call to `Future::poll`. If that poll runs past the deadline and returns `Ready`, `timeout`
returns `Ok`. If the poll never returns, the timeout never fires. So
`timeout(d, async { cpu_work() })` does not bound `cpu_work`.

Wrap the work as `timeout(d, spawn_blocking(work))` so the runtime can observe the deadline. This
bounds only how long the caller waits; the closure keeps running. When the work itself must stop,
pass a `CancellationToken` into it, check it between bounded work units, and cancel it on
`Elapsed`. Read the full example in
[references/cancellation-and-shutdown.md](references/cancellation-and-shutdown.md#cooperative-cancellation-of-cpu-work)
when you write such a loop.

## Runtime setup and foreign threads

Build one runtime explicitly and share it from a `static OnceLock<Runtime>`. Do not build a
runtime per session, per request, or per FFI call: each build creates a new thread pool. Set
`worker_threads` explicitly on mobile and embedded targets; the default is one thread per core.
Find a large future with `clippy::large_futures` (pedantic), and `Box::pin` it. A stack overflow
in a library that a host process loads can show as a plain SIGSEGV in the host crash report.

| Flavor | Use it for | Constraint |
|---|---|---|
| `multi_thread` | production I/O concurrency | `tokio::spawn` needs `Send + 'static` |
| `current_thread` | tests, and synchronous wrapper APIs that call `block_on` | `block_in_place` panics; `tokio::spawn` still needs `Send` |
| `LocalRuntime` (tokio 1.51+) | `!Send` tasks through `spawn_local`, with no `LocalSet` | cannot move between threads; build with `Builder::new_current_thread().enable_all().build_local(LocalOptions::default())` |

A foreign thread (a JNI call, a C callback, a platform service thread) cannot `.await`; it enters
the runtime through `block_on`. Do not call `block_on` for long-running work on a callback thread
that the host expects to return promptly. `block_on` polls the root future on the calling
thread, and `thread_stack_size` does not size that stack; give a thread you create for
`block_on` an explicit `stack_size`. Wrap a `block_on` body under an `extern "C"` entry in
`catch_unwind` when the host must get an error, because since Rust 1.81 a panic there aborts the
process. Read
[references/runtime-and-foreign-threads.md](references/runtime-and-foreign-threads.md) when you
write the shared-runtime initializer, set a thread stack size, or make a foreign thread start or
wait for async work. It has the hand-off and blocking shapes and the rules to adopt a received
file descriptor or socket. The
`rust-jni` and `ffi-error-progress-cancel` skills own the boundary contract, and
`uniffi-boundary` owns a generated boundary. Each applies when it is installed.

## Manual polling from a synchronous loop

Some designs pair a synchronous, step-driven engine (a userspace protocol stack, a simulation
tick loop, a hardware poll loop) with waker-driven tokio tasks. The two schedulers do not share
a waker. The bridge is a `poll_read` / `poll_write` call with `Context::from_waker(Waker::noop())`
(Rust 1.85), made from the loop tick. Never call such a helper from async code or wrap it in an
`async fn`: under a no-op waker, `Poll::Pending` means that no wake ever arrives, so the task
stalls for ever. Read [references/manual-poll-bridge.md](references/manual-poll-bridge.md) when
you write or extend the helper, map its EOF and write results, or drive an io_uring backend
from the loop.

## Related skills

Each applies when it is installed.

- `rust-callback-bounds`: `for<'a>` bounds on non-async `Fn` callbacks. This skill covers only
  the async half.
- `rust-event-loop-state`: sharing `&mut State` in a synchronous loop. This skill covers two
  futures over one state under a runtime.
- `rust-pin-projection`: what `Pin` enforces, `Unpin`, `PhantomPinned`, and structural
  projection. This skill covers polling and cancel safety.
- `rust-send-sync`: auto traits and why a type is or is not `Send`. `rust-unsafe` owns a manual
  `unsafe impl Send`.
- `rust-networking` and `rust-database`: cancellation at a network or transaction boundary.
- `rust-unsafe`: the `SAFETY:` comment convention, for example on SQE construction.
- `rust-debugging`: tokio-console, the `RuntimeMetrics` fallback for targets that cannot carry
  it, and async stack frames.
- `rust-observability`: the `tracing` subscriber setup.
- `rust-performance`: flamegraphs.
- `rust-test-tools`: loom for hand-rolled atomics.
