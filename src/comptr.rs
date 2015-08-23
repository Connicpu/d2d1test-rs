use std::mem::{transmute, zeroed};
use std::ops::{Deref, DerefMut};
use std::default::Default;
use winapi::*;

pub unsafe trait RefCounted {
    fn add_ref(&mut self);
    fn release(&mut self);
}

macro_rules! RefCountInterface {
    ($interface:ty) => {
        unsafe impl RefCounted for $interface {
            fn add_ref(&mut self) {
                unsafe { self.AddRef(); }
            }

            fn release(&mut self) {
                unsafe { self.Release(); }
            }
        }
    }
}

RefCountInterface!(IUnknown);
RefCountInterface!(ID2D1Factory);
RefCountInterface!(ID2D1RenderTarget);
RefCountInterface!(ID2D1HwndRenderTarget);
RefCountInterface!(ID2D1SolidColorBrush);
RefCountInterface!(IDWriteFactory);
RefCountInterface!(IDWriteTextFormat);

#[allow(raw_pointer_derive)]
#[derive(Debug)]
pub struct ComPtr<T: RefCounted> {
    pub ptr: *mut T,
}

impl<T: RefCounted> PartialEq for ComPtr<T> {
    fn eq(&self, other: &ComPtr<T>) -> bool {
        self.ptr == other.ptr
    }
}

impl<T: RefCounted> ComPtr<T> {
    pub fn uninit() -> ComPtr<T> {
        unsafe { ComPtr { ptr: zeroed() } }
    }

    pub fn wrap_existing(ptr: *mut T) -> ComPtr<T> {
        ComPtr {
            ptr: ptr
        }
    }

    pub fn addr(&mut self) -> &mut *mut T {
        &mut self.ptr
    }

    pub fn ptr(&self) -> *const T {
        self.ptr as *const T
    }

    pub fn ptr_mut(&mut self) -> *mut T {
        self.ptr
    }
}

impl<T: RefCounted> Default for ComPtr<T> {
    fn default() -> Self {
        ComPtr::uninit()
    }
}

impl<T: RefCounted> Drop for ComPtr<T> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            self.release();
        }
    }
}

impl<T: RefCounted> Clone for ComPtr<T> {
    fn clone(&self) -> ComPtr<T> {
        let mut other = ComPtr::wrap_existing(self.ptr);
        other.add_ref();
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
