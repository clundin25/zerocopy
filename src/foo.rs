// Copyright 2023 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! TODO

use core::{
    cmp::Ordering,
    fmt::{self, Debug, Display, Formatter},
    hash::Hash,
    mem::{self, ManuallyDrop},
    ops::{Deref, DerefMut},
    ptr,
};

use super::*;

/// An alternative to the standard library's [`MaybeUninit`] that supports
/// unsized types.
///
/// `MaybeUninit<T>` is identical to the standard library's `MaybeUninit` type
/// with the exception that it supports wrapping unsized types. Namely,
/// `MaybeUninit<T>` has the same layout as `T`, but it has no bit validity
/// constraints - any byte of a `MaybeUninit<T>` may have any value, including
/// uninitialized.
///
/// [`MaybeUninit`]: core::mem::MaybeUninit
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct MaybeUninit<T: AsMaybeUninit + ?Sized> {
    inner: T::MaybeUninit,
}

impl<T: AsMaybeUninit + ?Sized> Debug for MaybeUninit<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad(core::any::type_name::<Self>())
    }
}

impl<T: AsMaybeUninit + ?Sized> MaybeUninit<T> {
    /// Gets a shared reference to the contained value.
    ///
    /// # Safety
    ///
    /// Calling this when the content is not yet fully initialized causes
    /// undefined behavior. It is up to the caller to guarantee that `self` is
    /// really in an initialized state.
    pub unsafe fn assume_init_ref(&self) -> &T {
        let ptr = T::raw_from_maybe_uninit(&self.inner);
        // SAFETY: TODO
        unsafe { &*ptr }
    }

    /// Gets a mutable reference to the contained value.
    ///
    /// # Safety
    ///
    /// Calling this when the content is not yet fully initialized causes
    /// undefined behavior. It is up to the caller to guarantee that `self` is
    /// really in an initialized state.
    pub unsafe fn assume_init_mut(&mut self) -> &mut T {
        let ptr = T::raw_mut_from_maybe_uninit(&mut self.inner);
        // SAFETY: TODO
        unsafe { &mut *ptr }
    }
}

impl<T: Sized> MaybeUninit<T> {
    /// Creates a new `MaybeUninit<T>` in an uninitialized state.
    pub const fn uninit() -> MaybeUninit<T> {
        MaybeUninit { inner: mem::MaybeUninit::uninit() }
    }

    /// Extracts the value from the `MaybeUninit<T>` container.
    ///
    /// # Safety
    ///
    /// `assume_init` has the same safety requirements and guarantees as the
    /// standard library's [`MaybeUninit::assume_init`] method.
    ///
    /// [`MaybeUninit::assume_init`]: mem::MaybeUninit::assume_init
    pub const unsafe fn assume_init(self) -> T {
        // SAFETY: The caller has promised to uphold the safety invariants of
        // the exact function we're calling here. Since, for `T: Sized`,
        // `MaybeUninit<T>` is a `repr(transparent)` wrapper around
        // `mem::MaybeUninit<T>`, it is sound to treat `Self` as equivalent to a
        // `mem::MaybeUninit<T>` for the purposes of
        // `mem::MaybeUninit::assume_init`'s safety invariants.
        unsafe { self.inner.assume_init() }
    }
}

/// A type which can be wrapped in [`MaybeUninit`].
///
/// # Safety
///
/// The safety invariants on the associated `MaybeUninit` type must be
/// upheld.
pub unsafe trait AsMaybeUninit {
    /// A type which has the same layout as `Self`, but which has no validity
    /// constraints.
    ///
    /// Roughly speaking, this type is equivalent to what the standard library's
    /// [`MaybeUninit<Self>`] would be if `MaybeUninit` supported unsized types.
    ///
    /// # Safety
    ///
    /// For `T: AsMaybeUninit`, the following must hold:
    /// - Given `m: T::MaybeUninit`, it is sound to write any byte value,
    ///   including an uninitialized byte, at any byte offset in `m`
    /// - `T` and `T::MaybeUninit` have the same alignment requirement
    /// - It is valid to use an `as` cast to convert a `t: *const T` to a `m:
    ///   *const T::MaybeUninit` and vice-versa (and likewise for `*mut T`/`*mut
    ///   T::MaybeUninit`). Regardless of which direction the conversion was
    ///   performed, the sizes of the pointers' referents are always equal (in
    ///   terms of an API which is not yet stable, `size_of_val_raw(t) ==
    ///   size_of_val_raw(m)`).
    /// - `T::MaybeUninit` contains [`UnsafeCell`]s at exactly the same byte
    ///   ranges that `T` does.
    ///
    /// [`MaybeUninit<Self>`]: core::mem::MaybeUninit
    /// [`UnsafeCell`]: core::cell::UnsafeCell
    type MaybeUninit: ?Sized;

    /// Converts a const pointer at the type level.
    ///
    /// # Safety
    ///
    /// Callers may assume that the memory region addressed by the return value
    /// is the same as that addressed by the argument, and that both the return
    /// value and the argument have the same provenance.
    fn raw_from_maybe_uninit(maybe_uninit: *const Self::MaybeUninit) -> *const Self;

    /// Converts a mut pointer at the type level.
    ///
    /// # Safety
    ///
    /// Callers may assume that the memory region addressed by the return value
    /// is the same as that addressed by the argument, and that both the return
    /// value and the argument have the same provenance.
    fn raw_mut_from_maybe_uninit(maybe_uninit: *mut Self::MaybeUninit) -> *mut Self;
}

// SAFETY: See safety comment on `MaybeUninit`.
unsafe impl<T: Sized> AsMaybeUninit for T {
    // SAFETY:
    // - `MaybeUninit` has no validity requirements, so it is sound to write any
    //   byte value, including an uninitialized byte, at any offset.
    // - `MaybeUninit<T>` has the same layout as `T`, so they have the same
    //   alignment requirement. For the same reason, their sizes are equal.
    // - Since their sizes are equal, raw pointers to both types are thin
    //   pointers, and thus can be converted using as casts. For the same
    //   reason, the sizes of these pointers' referents are always equal.
    // - `MaybeUninit<T>` has the same field offsets as `T`, and so it contains
    //   `UnsafeCell`s at exactly the same byte ranges as `T`.
    type MaybeUninit = mem::MaybeUninit<T>;

    fn raw_from_maybe_uninit(maybe_uninit: *const mem::MaybeUninit<T>) -> *const T {
        maybe_uninit.cast::<T>()
    }

    fn raw_mut_from_maybe_uninit(maybe_uninit: *mut mem::MaybeUninit<T>) -> *mut T {
        maybe_uninit.cast::<T>()
    }
}

// SAFETY: See safety comment on `MaybeUninit`.
unsafe impl<T: Sized> AsMaybeUninit for [T] {
    // SAFETY:
    // - `MaybeUninit` has no bit validity requirements and `[U]` has the same
    //   bit validity requirements as `U`, so `[MaybeUninit<T>]` has no bit
    //   validity requirements. Thus, it is sound to write any byte value,
    //   including an uninitialized byte, at any byte offset.
    // - Since `MaybeUninit<T>` has the same layout as `T`, and `[U]` has the
    //   same alignment as `U`, `[MaybeUninit<T>]` has the same alignment as
    //   `[T]`.
    // - `[T]` and `[MaybeUninit<T>]` are both slice types, and so pointers can
    //   be converted using an `as` cast. Since `T` and `MaybeUninit<T>` have
    //   the same size, and since such a cast preserves the number of elements
    //   in the slice, the referent slices themselves will have the same size.
    // - `MaybeUninit<T>` has the same field offsets as `[T]`, and so it
    //   contains `UnsafeCell`s at exactly the same byte ranges as `[T]`.
    type MaybeUninit = [mem::MaybeUninit<T>];

    fn raw_from_maybe_uninit(maybe_uninit: *const [mem::MaybeUninit<T>]) -> *const [T] {
        maybe_uninit as *const [T]
    }

    fn raw_mut_from_maybe_uninit(maybe_uninit: *mut [mem::MaybeUninit<T>]) -> *mut [T] {
        maybe_uninit as *mut [T]
    }
}

// SAFETY: See safety comment on `MaybeUninit`.
unsafe impl AsMaybeUninit for str {
    // SAFETY: `str` has the same layout as `[u8]`. Thus, the same safety
    // argument for `<[u8] as AsMaybeUninit>::MaybeUninit` applies here.
    type MaybeUninit = <[u8] as AsMaybeUninit>::MaybeUninit;

    fn raw_from_maybe_uninit(
        maybe_uninit: *const <[u8] as AsMaybeUninit>::MaybeUninit,
    ) -> *const str {
        maybe_uninit as *const str
    }

    fn raw_mut_from_maybe_uninit(
        maybe_uninit: *mut <[u8] as AsMaybeUninit>::MaybeUninit,
    ) -> *mut str {
        maybe_uninit as *mut str
    }
}

// SAFETY: See safety comment on `MaybeUninit`.
unsafe impl<T: Sized> AsMaybeUninit for MaybeUninit<[T]> {
    // SAFETY: `MaybeUninit<[T]>` is a `repr(transparent)` wrapper around
    // `[T::MaybeUninit]`. Thus:
    // - Given `m: Self::MaybeUninit = [T::MaybeUninit]`, it is sound to write
    //   any byte value, including an uninitialized byte, at any byte offset in
    //   `m` because that is already required of `T::MaybeUninit`, and thus of
    //   [`T::MaybeUninit`]
    // - `Self` and `[T::MaybeUninit]` have the same representation, and so:
    //   - Alignments are equal
    //   - Pointer casts are valid, and sizes of referents of both pointer types
    //     are equal.
    // - `Self::MaybeUninit = [T::MaybeUninit]` contains `UnsafeCell`s at
    //   exactly the same byte ranges that `Self` does because `Self` has the
    //   same bit validity as `[T::MaybeUninit]`.
    type MaybeUninit = [<T as AsMaybeUninit>::MaybeUninit];

    fn raw_from_maybe_uninit(
        maybe_uninit: *const [<T as AsMaybeUninit>::MaybeUninit],
    ) -> *const MaybeUninit<[T]> {
        maybe_uninit as *const MaybeUninit<[T]>
    }

    fn raw_mut_from_maybe_uninit(
        maybe_uninit: *mut [<T as AsMaybeUninit>::MaybeUninit],
    ) -> *mut MaybeUninit<[T]> {
        maybe_uninit as *mut MaybeUninit<[T]>
    }
}

/// A type with no alignment requirement.
///
/// An `Unalign` wraps a `T`, removing any alignment requirement. `Unalign<T>`
/// has the same size and bit validity as `T`, but not necessarily the same
/// alignment [or ABI]. This is useful if a type with an alignment requirement
/// needs to be read from a chunk of memory which provides no alignment
/// guarantees.
///
/// Since `Unalign` has no alignment requirement, the inner `T` may not be
/// properly aligned in memory. There are five ways to access the inner `T`:
/// - by value, using [`get`] or [`into_inner`]
/// - by reference inside of a callback, using [`update`]
/// - fallibly by reference, using [`try_deref`] or [`try_deref_mut`]; these can
///   fail if the `Unalign` does not satisfy `T`'s alignment requirement at
///   runtime
/// - unsafely by reference, using [`deref_unchecked`] or
///   [`deref_mut_unchecked`]; it is the caller's responsibility to ensure that
///   the `Unalign` satisfies `T`'s alignment requirement
/// - (where `T: Unaligned`) infallibly by reference, using [`Deref::deref`] or
///   [`DerefMut::deref_mut`]
///
/// [or ABI]: https://github.com/google/zerocopy/issues/164
/// [`get`]: Unalign::get
/// [`into_inner`]: Unalign::into_inner
/// [`update`]: Unalign::update
/// [`try_deref`]: Unalign::try_deref
/// [`try_deref_mut`]: Unalign::try_deref_mut
/// [`deref_unchecked`]: Unalign::deref_unchecked
/// [`deref_mut_unchecked`]: Unalign::deref_mut_unchecked
// NOTE: This type is sound to use with types that need to be dropped. The
// reason is that the compiler-generated drop code automatically moves all
// values to aligned memory slots before dropping them in-place. This is not
// well-documented, but it's hinted at in places like [1] and [2]. However, this
// also means that `T` must be `Sized`; unless something changes, we can never
// support unsized `T`. [3]
//
// [1] https://github.com/rust-lang/rust/issues/54148#issuecomment-420529646
// [2] https://github.com/google/zerocopy/pull/126#discussion_r1018512323
// [3] https://github.com/google/zerocopy/issues/209
#[allow(missing_debug_implementations)]
#[derive(Default, Copy)]
#[cfg_attr(any(feature = "derive", test), derive(FromZeroes, FromBytes, AsBytes, Unaligned))]
#[repr(C, packed)]
pub struct Unalign<T>(T);

safety_comment! {
    /// SAFETY:
    /// - `Unalign<T>` is `repr(packed)`, so it is unaligned regardless of the
    ///   alignment of `T`, and so we don't require that `T: Unaligned`
    /// - `Unalign<T>` has the same bit validity as `T`, and so it is
    ///   `FromZeroes`, `FromBytes`, or `AsBytes` exactly when `T` is as well.
    impl_or_verify!(T => Unaligned for Unalign<T>);
    impl_or_verify!(T: FromZeroes => FromZeroes for Unalign<T>);
    impl_or_verify!(T: FromBytes => FromBytes for Unalign<T>);
    impl_or_verify!(T: AsBytes => AsBytes for Unalign<T>);
}

// Note that `Unalign: Clone` only if `T: Copy`. Since the inner `T` may not be
// aligned, there's no way to safely call `T::clone`, and so a `T: Clone` bound
// is not sufficient to implement `Clone` for `Unalign`.
impl<T: Copy> Clone for Unalign<T> {
    fn clone(&self) -> Unalign<T> {
        *self
    }
}

impl<T> Unalign<T> {
    /// Constructs a new `Unalign`.
    pub const fn new(val: T) -> Unalign<T> {
        Unalign(val)
    }

    /// Consumes `self`, returning the inner `T`.
    pub const fn into_inner(self) -> T {
        // Use this instead of `mem::transmute` since the latter can't tell
        // that `Unalign<T>` and `T` have the same size.
        #[repr(C)]
        union Transmute<T> {
            u: ManuallyDrop<Unalign<T>>,
            t: ManuallyDrop<T>,
        }

        // SAFETY: Since `Unalign` is `#[repr(C, packed)]`, it has the same
        // layout as `T`. `ManuallyDrop<U>` is guaranteed to have the same
        // layout as `U`, and so `ManuallyDrop<Unalign<T>>` has the same layout
        // as `ManuallyDrop<T>`. Since `Transmute<T>` is `#[repr(C)]`, its `t`
        // and `u` fields both start at the same offset (namely, 0) within the
        // union.
        //
        // We do this instead of just destructuring in order to prevent
        // `Unalign`'s `Drop::drop` from being run, since dropping is not
        // supported in `const fn`s.
        //
        // TODO(https://github.com/rust-lang/rust/issues/73255): Destructure
        // instead of using unsafe.
        unsafe { ManuallyDrop::into_inner(Transmute { u: ManuallyDrop::new(self) }.t) }
    }

    /// Attempts to return a reference to the wrapped `T`, failing if `self` is
    /// not properly aligned.
    ///
    /// If `self` does not satisfy `mem::align_of::<T>()`, then it is unsound to
    /// return a reference to the wrapped `T`, and `try_deref` returns `None`.
    ///
    /// If `T: Unaligned`, then `Unalign<T>` implements [`Deref`], and callers
    /// may prefer [`Deref::deref`], which is infallible.
    pub fn try_deref(&self) -> Option<&T> {
        if !crate::util::aligned_to::<_, T>(self) {
            return None;
        }

        // SAFETY: `deref_unchecked`'s safety requirement is that `self` is
        // aligned to `align_of::<T>()`, which we just checked.
        unsafe { Some(self.deref_unchecked()) }
    }

    /// Attempts to return a mutable reference to the wrapped `T`, failing if
    /// `self` is not properly aligned.
    ///
    /// If `self` does not satisfy `mem::align_of::<T>()`, then it is unsound to
    /// return a reference to the wrapped `T`, and `try_deref_mut` returns
    /// `None`.
    ///
    /// If `T: Unaligned`, then `Unalign<T>` implements [`DerefMut`], and
    /// callers may prefer [`DerefMut::deref_mut`], which is infallible.
    pub fn try_deref_mut(&mut self) -> Option<&mut T> {
        if !crate::util::aligned_to::<_, T>(&*self) {
            return None;
        }

        // SAFETY: `deref_mut_unchecked`'s safety requirement is that `self` is
        // aligned to `align_of::<T>()`, which we just checked.
        unsafe { Some(self.deref_mut_unchecked()) }
    }

    /// Returns a reference to the wrapped `T` without checking alignment.
    ///
    /// If `T: Unaligned`, then `Unalign<T>` implements[ `Deref`], and callers
    /// may prefer [`Deref::deref`], which is safe.
    ///
    /// # Safety
    ///
    /// If `self` does not satisfy `mem::align_of::<T>()`, then
    /// `self.deref_unchecked()` may cause undefined behavior.
    pub const unsafe fn deref_unchecked(&self) -> &T {
        // SAFETY: `Unalign<T>` is `repr(transparent)`, so there is a valid `T`
        // at the same memory location as `self`. It has no alignment guarantee,
        // but the caller has promised that `self` is properly aligned, so we
        // know that it is sound to create a reference to `T` at this memory
        // location.
        //
        // We use `mem::transmute` instead of `&*self.get_ptr()` because
        // dereferencing pointers is not stable in `const` on our current MSRV
        // (1.56 as of this writing).
        unsafe { mem::transmute(self) }
    }

    /// Returns a mutable reference to the wrapped `T` without checking
    /// alignment.
    ///
    /// If `T: Unaligned`, then `Unalign<T>` implements[ `DerefMut`], and
    /// callers may prefer [`DerefMut::deref_mut`], which is safe.
    ///
    /// # Safety
    ///
    /// If `self` does not satisfy `mem::align_of::<T>()`, then
    /// `self.deref_mut_unchecked()` may cause undefined behavior.
    pub unsafe fn deref_mut_unchecked(&mut self) -> &mut T {
        // SAFETY: `self.get_mut_ptr()` returns a raw pointer to a valid `T` at
        // the same memory location as `self`. It has no alignment guarantee,
        // but the caller has promised that `self` is properly aligned, so we
        // know that the pointer itself is aligned, and thus that it is sound to
        // create a reference to a `T` at this memory location.
        unsafe { &mut *self.get_mut_ptr() }
    }

    /// Gets an unaligned raw pointer to the inner `T`.
    ///
    /// # Safety
    ///
    /// The returned raw pointer is not necessarily aligned to
    /// `align_of::<T>()`. Most functions which operate on raw pointers require
    /// those pointers to be aligned, so calling those functions with the result
    /// of `get_ptr` will be undefined behavior if alignment is not guaranteed
    /// using some out-of-band mechanism. In general, the only functions which
    /// are safe to call with this pointer are those which are explicitly
    /// documented as being sound to use with an unaligned pointer, such as
    /// [`read_unaligned`].
    ///
    /// [`read_unaligned`]: core::ptr::read_unaligned
    pub const fn get_ptr(&self) -> *const T {
        ptr::addr_of!(self.0)
    }

    /// Gets an unaligned mutable raw pointer to the inner `T`.
    ///
    /// # Safety
    ///
    /// The returned raw pointer is not necessarily aligned to
    /// `align_of::<T>()`. Most functions which operate on raw pointers require
    /// those pointers to be aligned, so calling those functions with the result
    /// of `get_ptr` will be undefined behavior if alignment is not guaranteed
    /// using some out-of-band mechanism. In general, the only functions which
    /// are safe to call with this pointer are those which are explicitly
    /// documented as being sound to use with an unaligned pointer, such as
    /// [`read_unaligned`].
    ///
    /// [`read_unaligned`]: core::ptr::read_unaligned
    // TODO(https://github.com/rust-lang/rust/issues/57349): Make this `const`.
    pub fn get_mut_ptr(&mut self) -> *mut T {
        ptr::addr_of_mut!(self.0)
    }

    /// Sets the inner `T`, dropping the previous value.
    // TODO(https://github.com/rust-lang/rust/issues/57349): Make this `const`.
    pub fn set(&mut self, t: T) {
        *self = Unalign::new(t);
    }

    /// Updates the inner `T` by calling a function on it.
    ///
    /// If [`T: Unaligned`], then `Unalign<T>` implements [`DerefMut`], and that
    /// impl should be preferred over this method when performing updates, as it
    /// will usually be faster and more ergonomic.
    ///
    /// For large types, this method may be expensive, as it requires copying
    /// `2 * size_of::<T>()` bytes. \[1\]
    ///
    /// \[1\] Since the inner `T` may not be aligned, it would not be sound to
    /// invoke `f` on it directly. Instead, `update` moves it into a
    /// properly-aligned location in the local stack frame, calls `f` on it, and
    /// then moves it back to its original location in `self`.
    ///
    /// [`T: Unaligned`]: Unaligned
    pub fn update<O, F: FnOnce(&mut T) -> O>(&mut self, f: F) -> O {
        // On drop, this moves `copy` out of itself and uses `ptr::write` to
        // overwrite `slf`.
        struct WriteBackOnDrop<T> {
            copy: ManuallyDrop<T>,
            slf: *mut Unalign<T>,
        }

        impl<T> Drop for WriteBackOnDrop<T> {
            fn drop(&mut self) {
                // SAFETY: See inline comments.
                unsafe {
                    // SAFETY: We never use `copy` again as required by
                    // `ManuallyDrop::take`.
                    let copy = ManuallyDrop::take(&mut self.copy);
                    // SAFETY: `slf` is the raw pointer value of `self`. We know
                    // it is valid for writes and properly aligned because
                    // `self` is a mutable reference, which guarantees both of
                    // these properties.
                    ptr::write(self.slf, Unalign::new(copy));
                }
            }
        }

        // SAFETY: We know that `self` is valid for reads, properly aligned, and
        // points to an initialized `Unalign<T>` because it is a mutable
        // reference, which guarantees all of these properties.
        //
        // Since `T: !Copy`, it would be unsound in the general case to allow
        // both the original `Unalign<T>` and the copy to be used by safe code.
        // We guarantee that the copy is used to overwrite the original in the
        // `Drop::drop` impl of `WriteBackOnDrop`. So long as this `drop` is
        // called before any other safe code executes, soundness is upheld.
        // While this method can terminate in two ways (by returning normally or
        // by unwinding due to a panic in `f`), in both cases, `write_back` is
        // dropped - and its `drop` called - before any other safe code can
        // execute.
        let copy = unsafe { ptr::read(self) }.into_inner();
        let mut write_back = WriteBackOnDrop { copy: ManuallyDrop::new(copy), slf: self };

        let ret = f(&mut write_back.copy);

        drop(write_back);
        ret
    }
}

impl<T: Copy> Unalign<T> {
    /// Gets a copy of the inner `T`.
    // TODO(https://github.com/rust-lang/rust/issues/57349): Make this `const`.
    pub fn get(&self) -> T {
        let Unalign(val) = *self;
        val
    }
}

impl<T: Unaligned> Deref for Unalign<T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: `deref_unchecked`'s safety requirement is that `self` is
        // aligned to `align_of::<T>()`. `T: Unaligned` guarantees that
        // `align_of::<T>() == 1`, and all pointers are one-aligned because all
        // addresses are divisible by 1.
        unsafe { self.deref_unchecked() }
    }
}

impl<T: Unaligned> DerefMut for Unalign<T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: `deref_mut_unchecked`'s safety requirement is that `self` is
        // aligned to `align_of::<T>()`. `T: Unaligned` guarantees that
        // `align_of::<T>() == 1`, and all pointers are one-aligned because all
        // addresses are divisible by 1.
        unsafe { self.deref_mut_unchecked() }
    }
}

impl<T: Unaligned + PartialOrd> PartialOrd<Unalign<T>> for Unalign<T> {
    fn partial_cmp(&self, other: &Unalign<T>) -> Option<Ordering> {
        PartialOrd::partial_cmp(self.deref(), other.deref())
    }
}

impl<T: Unaligned + Ord> Ord for Unalign<T> {
    fn cmp(&self, other: &Unalign<T>) -> Ordering {
        Ord::cmp(self.deref(), other.deref())
    }
}

impl<T: Unaligned + PartialEq> PartialEq<Unalign<T>> for Unalign<T> {
    fn eq(&self, other: &Unalign<T>) -> bool {
        PartialEq::eq(self.deref(), other.deref())
    }
}

impl<T: Unaligned + Eq> Eq for Unalign<T> {}

impl<T: Unaligned + Hash> Hash for Unalign<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.deref().hash(state);
    }
}

impl<T: Unaligned + Debug> Debug for Unalign<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.deref(), f)
    }
}

impl<T: Unaligned + Display> Display for Unalign<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self.deref(), f)
    }
}

#[cfg(test)]
mod tests {
    use core::panic::AssertUnwindSafe;

    use super::*;
    use crate::util::testutil::*;

    /// A `T` which is guaranteed not to satisfy `align_of::<A>()`.
    ///
    /// It must be the case that `align_of::<T>() < align_of::<A>()` in order
    /// fot this type to work properly.
    #[repr(C)]
    struct ForceUnalign<T, A> {
        // The outer struct is aligned to `A`, and, thanks to `repr(C)`, `t` is
        // placed at the minimum offset that guarantees its alignment. If
        // `align_of::<T>() < align_of::<A>()`, then that offset will be
        // guaranteed *not* to satisfy `align_of::<A>()`.
        _u: u8,
        t: T,
        _a: [A; 0],
    }

    impl<T, A> ForceUnalign<T, A> {
        const fn new(t: T) -> ForceUnalign<T, A> {
            ForceUnalign { _u: 0, t, _a: [] }
        }
    }

    #[test]
    fn test_unalign() {
        // Test methods that don't depend on alignment.
        let mut u = Unalign::new(AU64(123));
        assert_eq!(u.get(), AU64(123));
        assert_eq!(u.into_inner(), AU64(123));
        assert_eq!(u.get_ptr(), <*const _>::cast::<AU64>(&u));
        assert_eq!(u.get_mut_ptr(), <*mut _>::cast::<AU64>(&mut u));
        u.set(AU64(321));
        assert_eq!(u.get(), AU64(321));

        // Test methods that depend on alignment (when alignment is satisfied).
        let mut u: Align<_, AU64> = Align::new(Unalign::new(AU64(123)));
        assert_eq!(u.t.try_deref(), Some(&AU64(123)));
        assert_eq!(u.t.try_deref_mut(), Some(&mut AU64(123)));
        // SAFETY: The `Align<_, AU64>` guarantees proper alignment.
        assert_eq!(unsafe { u.t.deref_unchecked() }, &AU64(123));
        // SAFETY: The `Align<_, AU64>` guarantees proper alignment.
        assert_eq!(unsafe { u.t.deref_mut_unchecked() }, &mut AU64(123));
        *u.t.try_deref_mut().unwrap() = AU64(321);
        assert_eq!(u.t.get(), AU64(321));

        // Test methods that depend on alignment (when alignment is not
        // satisfied).
        let mut u: ForceUnalign<_, AU64> = ForceUnalign::new(Unalign::new(AU64(123)));
        assert_eq!(u.t.try_deref(), None);
        assert_eq!(u.t.try_deref_mut(), None);

        // Test methods that depend on `T: Unaligned`.
        let mut u = Unalign::new(123u8);
        assert_eq!(u.try_deref(), Some(&123));
        assert_eq!(u.try_deref_mut(), Some(&mut 123));
        assert_eq!(u.deref(), &123);
        assert_eq!(u.deref_mut(), &mut 123);
        *u = 21;
        assert_eq!(u.get(), 21);

        // Test that some `Unalign` functions and methods are `const`.
        const _UNALIGN: Unalign<u64> = Unalign::new(0);
        const _UNALIGN_PTR: *const u64 = _UNALIGN.get_ptr();
        const _U64: u64 = _UNALIGN.into_inner();
        // Make sure all code is considered "used".
        //
        // TODO(https://github.com/rust-lang/rust/issues/104084): Remove this
        // attribute.
        #[allow(dead_code)]
        const _: () = {
            let x: Align<_, AU64> = Align::new(Unalign::new(AU64(123)));
            // Make sure that `deref_unchecked` is `const`.
            //
            // SAFETY: The `Align<_, AU64>` guarantees proper alignment.
            let au64 = unsafe { x.t.deref_unchecked() };
            match au64 {
                AU64(123) => {}
                _ => unreachable!(),
            }
        };
    }

    #[test]
    fn test_unalign_update() {
        let mut u = Unalign::new(AU64(123));
        u.update(|a| a.0 += 1);
        assert_eq!(u.get(), AU64(124));

        // Test that, even if the callback panics, the original is still
        // correctly overwritten. Use a `Box` so that Miri is more likely to
        // catch any unsoundness (which would likely result in two `Box`es for
        // the same heap object, which is the sort of thing that Miri would
        // probably catch).
        let mut u = Unalign::new(Box::new(AU64(123)));
        let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
            u.update(|a| {
                a.0 += 1;
                panic!();
            })
        }));
        assert!(res.is_err());
        assert_eq!(u.into_inner(), Box::new(AU64(124)));
    }
}
