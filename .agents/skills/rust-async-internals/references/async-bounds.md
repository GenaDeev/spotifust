# Async Bounds and Types

Reference material for `SKILL.md`. Read the section that matches the bound or type that fails.

## Contents

- [Async closures and the AsyncFn family](#async-closures-and-the-asyncfn-family)
- [A single Fut parameter rejects every borrowing async fn](#a-single-fut-parameter-rejects-every-borrowing-async-fn)
- [AsyncFn carries no Send bound to the future](#asyncfn-carries-no-send-bound-to-the-future)
- [async fn in traits: Send and dyn](#async-fn-in-traits-send-and-dyn)
- [impl Trait (RPIT) captures in edition 2024](#impl-trait-rpit-captures-in-edition-2024)
- [Two futures over one state](#two-futures-over-one-state)
- [Why a future or an FFI type needs Pin](#why-a-future-or-an-ffi-type-needs-pin)

## Async closures and the AsyncFn family

Rust 1.85 (RFC 3668) stabilized `async ||` closures and the `AsyncFn` / `AsyncFnMut` /
`AsyncFnOnce` traits, in the prelude of every edition. `AsyncFn(&T)` replaces the boxed
higher-ranked shape. It uses no box and no allocation. `async ||` infers the borrow of a
captured value.

- Rust 1.85 and later: `F: AsyncFn(&str) -> R`, called with `async |s: &str| do_work(s).await`.
- Any stable release: `F: for<'a> Fn(&'a str) -> Pin<Box<dyn Future<Output = R> + 'a>>`,
  called with `|s| Box::pin(do_work(s))`.

Rules, once the crate MSRV is 1.85 or above:

1. For a new higher-ranked async bound, prefer `F: AsyncFn(Args) -> T` over
   `F: Fn(Args) -> Fut, Fut: Future<Output = T>`, as long as the future stays on the caller's
   task. It does not survive a `tokio::spawn`; see the next two sections.
2. Do not mass-rewrite existing `|x| async move { ... }` into `async |x|` in unrelated diffs.
   Migrate a site when you touch it for another reason, so `git blame` stays useful.
3. The `rust-callback-bounds` skill, when it is installed, owns the non-async `for<'a>` rules:
   a dependent output such as `-> &'a K` is legal in `Fn` sugar and needs no box.

An async closure always captures its input arguments, because the returned future can use them.
An ordinary closure does not capture an unused input. An async closure is also lending when the
future borrows a capture, or when it reads a by-value capture without a dereference. A lending
async closure does not implement `Fn` or `FnMut`; it implements only `FnOnce`. Do not add clones
until a type probe shows which call trait the exact closure needs.

## A single Fut parameter rejects every borrowing async fn

`fn f<Fut: Future>(cb: impl Fn(&T) -> Fut)` accepts no `async fn` that borrows its argument.
`async fn handle(r: &Request)` returns a different opaque type per region, and a single `Fut`
parameter is chosen once, outside the `for<'a>` binder. rustc 1.98.1 prints:

```text
error[E0308]: mismatched types
  |
6 | fn main() { add_reactor(handle); }
  |             ^^^^^^^^^^^^^^^^^^^ one type is more general than the other
  |
  = note: expected opaque type `impl for<'a> Future<Output = ()>`
             found opaque type `impl Future<Output = ()>`
  = note: distinct uses of `impl Trait` result in different opaque types
```

Two bounds accept it. Pick by whether the future must cross a task boundary:

```rust
use std::future::Future;
use std::pin::Pin;

struct Request { body: String }
async fn handle(r: &Request) { let _ = &r.body; }

// FIX A, Rust 1.85 and later: the future stays local to the caller.
fn reactor_local(_f: impl AsyncFn(&Request)) {}

// FIX B, any stable release: an `Fn` bound that also carries `Send`.
fn reactor_spawnable(
    _f: impl for<'a> Fn(&'a Request) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>,
) {}

fn main() {
    reactor_local(handle);
    reactor_local(async |r: &Request| { let _ = &r.body; });
    reactor_spawnable(|r| Box::pin(handle(r)));
}
```

Write `Pin<Box<dyn Future<..> + 'a>>`, not a bare `Box<dyn Future<..> + 'a>`. `Box<F>`
implements `Future` only for `F: Unpin`, and `dyn Future` is not `Unpin`, so the bare form
type-checks and then fails at the `.await` with
``error[E0277]: `dyn Future<Output = ()>` cannot be unpinned``.

## AsyncFn carries no Send bound to the future

`F: AsyncFn(&T) + Send + Sync + 'static` does not make the future that `F` returns `Send`. Those
bounds constrain the callable, not its output. The callback still fails to spawn, and
`tokio::spawn` prints an unnumbered error:

```text
error: future cannot be sent between threads safely
   |     tokio::spawn(async move { f(&req).await; });
   |     ^^^ future created by async block is not `Send`
   = help: within `{async block@src/main.rs:6:18: 6:28}`, the trait `Send` is
           not implemented for `<F as AsyncFnMut<(&Request,)>>::CallRefFuture<'_>`
note: required by a bound in `tokio::spawn`
```

A plain `T: Send` bound applied to the returned future prints the numbered form of the same
fact:

```text
error[E0277]: `<F as AsyncFnMut<(&Request,)>>::CallRefFuture<'_>` cannot be sent
              between threads safely
  = help: the trait `Send` is not implemented for
          `<F as AsyncFnMut<(&Request,)>>::CallRefFuture<'_>`
```

The returned future is `<F as AsyncFnMut<(&'a T,)>>::CallRefFuture<'a>`, an associated type of
the unstable `async_fn_traits` feature. You cannot name it on stable, so you cannot write
`for<'a> F::CallRefFuture<'a>: Send`. No stable bound reaches the future of an `AsyncFn`.

When the future must cross `tokio::spawn` and the callback must stay an `Fn` bound, take FIX B
above: `F: for<'a> Fn(&'a T) -> Pin<Box<dyn Future<Output = R> + Send + 'a>>`. One `Box::pin` per
call is the price. Use `AsyncFn` everywhere else.

When you control the callee's shape, a trait method that returns
`impl Future<Output = R> + Send` (RPITIT, stable since 1.75) carries `Send` and allocates
nothing. It does not add `'static`. A future that borrows its argument cannot go directly to
`tokio::spawn`:

```rust
use std::future::Future;
use std::sync::Arc;

struct Request { body: String }

trait Handler: Send + Sync + 'static {
    fn call(&self, r: &Request) -> impl Future<Output = usize> + Send;
}

struct Len;
impl Handler for Len {
    async fn call(&self, r: &Request) -> usize { r.body.len() }
}

fn spawn_call(
    handler: Arc<impl Handler>,
    request: Request,
) -> tokio::task::JoinHandle<usize> {
    tokio::spawn(async move {
        // The outer task owns both values. Create the borrowed future here.
        handler.call(&request).await
    })
}
```

`tokio::spawn(handler.call(&request))` fails because both borrows are local and the spawned
future must be `'static`. Move owned values or `Arc` handles into an outer `async move` block.
Then create and await the borrowed RPITIT future inside that block.

## async fn in traits: Send and dyn

`async fn` in traits is stable since Rust 1.75. Two hazards appear when you replace
`#[async_trait]`.

1. **No automatic `Send` bound.** `SKILL.md`, section "Send, 'static, and async bounds", states
   the rule. A public trait with `async fn` also triggers the `async_fn_in_trait` warning for
   this reason.
2. **Not dyn compatible.** A trait with an `async fn` or a `-> impl Future` method cannot be used
   as `dyn Trait`. Code that used `Box<dyn MyTrait>` under `#[async_trait]`, which boxes the
   futures, fails with E0038. Current rustc says "is not dyn compatible"; a search of the build
   log for the older term "object safe" finds nothing.

`#[trait_variant::make(MyTraitSend: Send)]` fixes only the first hazard. It generates a variant
whose methods return `impl Future<Output = T> + Send`, and that variant is still not dyn
compatible. This block is that expansion, and it fails with E0038 (trait-variant 0.1.3 on rustc
1.98.1 gives the same error):

```rust,compile_fail,E0038
use std::future::Future;

// The shape that `#[trait_variant::make(SvcSend: Send)]` generates.
trait SvcSend: Send {
    fn call(&self) -> impl Future<Output = u32> + Send;
}

fn store(_svc: Box<dyn SvcSend>) {}

fn main() {}
```

For `dyn`, pick one of these:

- keep `#[async_trait]`, which boxes every call;
- declare the method as `fn call(&self) -> Pin<Box<dyn Future<Output = u32> + Send + '_>>`;
- use the third-party `dynosaur` crate, which generates a boxed `dyn` wrapper next to the
  native trait.

Return type notation (`T::call(..): Send`) would bound the future of a generic call, but it is
nightly-only (tracking issue #109417). Stable rustc rejects it:

```rust,compile_fail,E0658
trait Svc {
    async fn call(&self) -> u32;
}

fn spawnable<T>(_svc: T)
where
    T: Svc + Send + 'static,
    T::call(..): Send,
{
}

fn main() {}
```

Audit every `Box<dyn>` and every `tokio::spawn` use site before you replace `#[async_trait]`.

## impl Trait (RPIT) captures in edition 2024

In edition 2021 and earlier, return-position `impl Trait` did not capture a lifetime parameter
unless you listed it. In edition 2024, it captures every in-scope lifetime. A function that
returned a `'static`-compatible future in edition 2021 can become non-`'static` after the
migration, because the return type now captures a lifetime from an input reference.

Symptom: a method that takes `&self` and returns `impl Future` now captures `'_`, and every
`tokio::spawn(obj.method())` call site breaks.

Precise capturing, `use<..>` (Rust 1.82; in trait methods since 1.87), states which lifetimes and
type parameters the opaque type captures. `use<>` with an empty list captures nothing, so it
compiles only when the body holds no borrow. A future that keeps a reference must name the
lifetime, and it is then not `'static`:

```rust
// The future holds `data`, so it captures 'a. It cannot be spawned.
fn borrows<'a>(data: &'a str) -> impl Future<Output = usize> + use<'a> {
    async move { data.len() }
}
```

To get a `'static` future back, remove the borrow rather than the capture. Take ownership, and
`use<>` then holds:

```rust
// Captures nothing, so the future is 'static and `tokio::spawn` accepts it.
fn owns(data: String) -> impl Future<Output = usize> + use<> {
    async move { data.len() }
}
```

`use<>` on the borrowing form does not make it `'static`; it fails with
`error[E0700]: hidden type for ... captures lifetime that does not appear in bounds`. The
lifetime is a property of what the body holds. The capture list only declares it.

The `impl_trait_overcaptures` lint (in the `rust-2024-compatibility` group) flags affected sites
before the migration. The `cargo-workflows` skill, when it is installed, owns the
`cargo fix --edition` workflow; inspect every RPIT diff it makes.

## Two futures over one state

A captured `&mut State` inside an async block lives for the whole life of the future, from the
first poll to completion. Two concurrent futures therefore cannot both hold `&mut State`:

```rust,compile_fail,E0499
struct State { n: u32 }

impl State {
    fn handle_event(&mut self, ev: u32) { self.n += ev; }
}

async fn run() {
    let mut state = State { n: 0 };
    let f1 = async { state.handle_event(1) };
    let f2 = async { state.handle_event(2) }; // second mutable borrow of `state`
    tokio::join!(f1, f2);
}

fn main() {}
```

When `state` is itself a `&mut State` binding, such as a parameter, rustc 1.98.1 reports E0524
("two closures require unique access to `*state` at the same time") for the same shape.

Correct approaches, in order of preference:

1. **Split the state by field at the call site.** Two concurrently polled futures can share one
   state struct. The borrow a future captures is the borrow made at the call site, and disjoint
   field borrows stay disjoint inside futures. Try this before any channel, lock, or `RefCell`.

   ```rust
   struct State { seen: u32, out: Vec<u32> }

   async fn bump(n: &mut u32) { *n += 1; }
   async fn record(v: &mut Vec<u32>) { v.push(1); }

   let mut state = State { seen: 0, out: Vec::new() };
   // Two disjoint field borrows, both futures polled concurrently.
   tokio::join!(bump(&mut state.seen), record(&mut state.out));
   assert_eq!(state.seen, 1);
   ```

   It fails only when both futures need the same field, or when either takes `&mut State`
   whole. `tokio::join!(f(&mut st), f(&mut st))` is E0499 whatever the fields are.
2. **Single-task ownership.** One task owns `State`. Every other task talks to it through `mpsc`
   channels. Prefer this on any hot path.
3. **`Arc<Mutex<State>>`.** Correct, but it serializes access. Acceptable for low-contention
   configuration state; not on a per-packet or per-frame path.
4. **`Rc<RefCell<State>>`.** Valid only for futures that never need `Send`: tasks started with
   `spawn_local` on a `LocalRuntime` (tokio 1.51+) or inside a `LocalSet`. `tokio::spawn` needs
   `Send` on every runtime flavor. Do not hold a `RefCell` borrow across `.await`.

No route hands a `&mut State` to a future. `Context::ext` is nightly, and it is empty under
tokio. A pointer smuggled through `Waker::data()` breaks under `FuturesUnordered`. A coroutine
resume argument is not higher-ranked. The `rust-event-loop-state` skill, when it is installed,
holds the diagnostics, in its section "The three escapes, and why none works".

## Why a future or an FFI type needs Pin

`Pin<&mut T>` restricts `T` only when `T` is `!Unpin`. On an `Unpin` target, `Pin::new`,
`Pin::get_mut`, and `DerefMut` are all safe, so the pin enforces nothing. The
`rust-pin-projection` skill, when it is installed, owns `Pin`, `Unpin`, `PhantomPinned`, and
structural projection. This section covers only why a future or an FFI type needs a pin at all.

Every `async fn` and every `async {}` future is `!Unpin`, whatever the body holds. There is no
per-body analysis: even `async fn trivial() -> u32 { 1 }` fails an `Unpin` bound with
``error[E0277]: `{async fn body of trivial()}` cannot be unpinned``. A generic helper that polls
a caller-supplied future must therefore take `Pin<&mut F>`. An `F: Unpin` bound forces every
caller through `Box::pin` or `std::pin::pin!`.

A value needs `Pin` when:

- it is self-referential: a field holds a pointer to another field of the same value, as in the
  state machine of an async block;
- it is a C++ object with a non-trivial move constructor that Rust cannot call.

`cxx`-generated bindings expose C++ types as `Pin<&mut CppType>`. Use that generated contract as
`cxx` documents it. Do not generalize it to an opaque pointer that a C library allocates.
`Pin<Box<T>>` claims that Rust owns a `T` allocation and frees it with the Rust allocator. That
is false for a C-owned allocation.

Store a C-owned handle as `NonNull<Opaque>`. Call the matching C destructor in `Drop`. Moving the
Rust wrapper does not move the C allocation:

```rust
use std::ptr::NonNull;

#[repr(C)]
pub struct OpaqueHandle {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn handle_destroy(handle: *mut OpaqueHandle);
}

pub struct OwnedHandle {
    ptr: NonNull<OpaqueHandle>,
}

impl OwnedHandle {
    /// # Safety
    /// For a non-null `ptr`, the caller must transfer exclusive ownership of a
    /// live handle from the matching C constructor. No second owner or wrapper
    /// may exist. Foreign code must not destroy the handle or retain an access
    /// that can outlive this wrapper.
    pub unsafe fn from_raw(ptr: *mut OpaqueHandle) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self { ptr })
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: `from_raw` transfers the only ownership claim to this value.
        // Drop runs once, after all mediated foreign accesses have ended.
        unsafe { handle_destroy(self.ptr.as_ptr()) };
    }
}
```

Expose foreign operations as methods that borrow `OwnedHandle`. Do not expose a second owning
constructor. If C retains the pointer after a method returns, add an explicit
unregister-and-join step before `Drop` can run.

Do not add `Send` or `Sync` unless the C API documents the same thread-safety contract. Use `Pin`
only for Rust-owned values whose address-stability contract requires it, or when the generated
binding API requires a pinned reference. The `rust-pin-projection` skill, when it is installed,
owns the `SAFETY:` wording for `get_unchecked_mut` and the rule against pinning a stack binding
that can still be named.
