use std::mem::transmute;
use std::ops::{Deref, DerefMut};
use winapi::*;

pub unsafe trait RefCounted {
    fn add_ref(&mut self);
    fn release(&mut self);
}

unsafe impl<T> RefCounted for T {
    fn add_ref(&mut self) {
        unsafe {
            let iunknown: &mut &mut IUnknown = transmute(self);
            iunknown.AddRef();
        }
    }

    fn release(&mut self) {
        unsafe {
            let iunknown: &mut &mut IUnknown = transmute(self);
            iunknown.Release();
        }
    }
}

#[allow(raw_pointer_derive)]
#[derive(Debug)]
pub struct ComPtr<T: RefCounted> {
    ptr: *mut T,
}

impl<T: RefCounted> ComPtr<T> {
    pub fn wrap_existing(ptr: *mut T) -> ComPtr<T> {
        ComPtr {
            ptr: ptr
        }
    }
}

impl<T: RefCounted> Drop for ComPtr<T> {
    fn drop(&mut self) {
        self.ptr.release();
    }
}

impl<T: RefCounted> Clone for ComPtr<T> {
    fn clone(&self) -> ComPtr<T> {
        let mut other = ComPtr::wrap_existing(self.ptr);
        other.ptr.add_ref();
        other
    }
}

impl<T: RefCounted> Deref for ComPtr<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { transmute(self.ptr) }
    }
}

impl<T: RefCounted> DerefMut for ComPtr<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { transmute(self.ptr) }
    }
}
