#![cfg_attr(not(test), no_std)]

use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::*;

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_waker, wake, wake_by_ref, drop_waker);

pub fn block_on<T>(mut fut: impl Future<Output = T>) -> T {
    let woke = AtomicBool::new(false);

    let raw_waker = RawWaker::new(&woke as *const _ as _, &VTABLE);
    let waker = unsafe { Waker::from_raw(raw_waker) };
    let mut cx = Context::from_waker(&waker);

    loop {
        let pinned_fut = unsafe { Pin::new_unchecked(&mut fut) };

        match pinned_fut.poll(&mut cx) {
            Poll::Ready(x) => break x,
            _ => {
                while !woke.swap(false, Ordering::AcqRel) {
                    core::hint::spin_loop();
                }
            }
        }
    }
}

unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
    RawWaker::new(ptr, &VTABLE)
}

unsafe fn wake(ptr: *const ()) {
    let ptr = &*(ptr as *const AtomicBool);
    ptr.store(true, Ordering::Release);
}

unsafe fn wake_by_ref(ptr: *const ()) {
    wake(ptr);
}

unsafe fn drop_waker(_ptr: *const ()) {}

#[cfg(test)]
mod tests {
    use super::block_on;

    #[test]
    fn simple() {
        let result = async { 2 + 2 };
        assert_eq!(block_on(result), 4);
    }

    #[test]
    fn complicated() {
        let fut = async_calc();
        assert_eq!(block_on(fut), 4);
    }

    async fn async_calc() -> i32 {
        2 + 2
    }

    #[test]
    fn more_complicated() {
        let fut = async_calc_complicated();
        assert_eq!(block_on(fut), 4);
    }

    async fn async_calc_complicated() -> i32 {
        async_sub1().await + async_sub2().await
    }

    async fn async_sub1() -> i32 {
        2
    }

    async fn async_sub2() -> i32 {
        2
    }
}
