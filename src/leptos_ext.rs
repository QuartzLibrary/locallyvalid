use either::Either;
use instant::Instant;
use leptos::{
    create_memo, create_render_effect, create_rw_signal, untrack, RwSignal, Signal, SignalGet,
    SignalGetUntracked, SignalSet, SignalSetter, SignalUpdate, SignalWith, SignalWithUntracked,
};
use std::{
    cell::RefCell,
    fmt,
    future::Future,
    mem,
    ops::{Deref, DerefMut, Not},
    rc::Rc,
    time::Duration,
};

#[derive(Debug, Clone)]
struct SharedBox<T> {
    inner: Rc<RefCell<T>>,
}
impl<T> SharedBox<T> {
    fn new(v: T) -> Self {
        Self {
            inner: Rc::new(RefCell::new(v)),
        }
    }
    fn get(&self) -> T
    where
        T: Copy,
    {
        *self.inner.borrow()
    }
    fn get_cloned(&self) -> T
    where
        T: Clone,
    {
        self.inner.borrow().clone()
    }
    fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        f(&*self.inner.borrow())
    }

    fn set(&self, to: T) {
        *self.inner.borrow_mut() = to;
    }
    fn from_to(&self, from: &T, to: T)
    where
        T: fmt::Debug + Clone + PartialEq,
    {
        assert_eq!(from, &self.get_cloned());
        self.set(to);
    }
}

pub trait ReadSignalExt:
    SignalWith<Value = <Self as ReadSignalExt>::Inner>
    + SignalWithUntracked<Value = <Self as ReadSignalExt>::Inner>
    + Clone
    + 'static
{
    type Inner;

    #[track_caller]
    fn map<U>(&self, f: impl FnMut(&Self::Inner) -> U + 'static) -> Signal<U> {
        let self_ = self.clone();
        let f = RefCell::new(f);
        (move || self_.with(|v| untrack(|| f.borrow_mut()(v)))).into()
    }
    #[track_caller]
    fn map_dedup<U>(&self, f: impl FnMut(&Self::Inner) -> U + 'static) -> Signal<U>
    where
        U: PartialEq,
    {
        let self_ = self.clone();
        let f = RefCell::new(f);
        create_memo(move |_| self_.with(|v| untrack(|| f.borrow_mut()(v)))).into()
    }
    #[track_caller]
    fn map_window<U>(
        &self,
        mut f: impl FnMut(Option<&Self::Inner>, &Self::Inner) -> U + 'static,
    ) -> Signal<U>
    where
        Self::Inner: Clone,
    {
        let current_value = self.with_untracked(Clone::clone);
        let ret = create_rw_signal(untrack(|| (f(None, &current_value))));

        let mut old = current_value;
        self.for_each_after_first(move |new| {
            ret.set(untrack(|| f(Some(&old), new)));
            old = new.clone();
        });
        ret.read_only().into()
    }

    #[track_caller]
    fn dedup(&self) -> Signal<Self::Inner>
    where
        Self::Inner: PartialEq + Clone,
    {
        self.map_dedup(Self::Inner::clone)
    }
    /// Will start with the same value, but then any values matching the provided closure will be *skipped*.
    #[track_caller]
    fn skip_if(&self, mut f: impl FnMut(&Self::Inner) -> bool + 'static) -> Signal<Self::Inner>
    where
        Self::Inner: Clone,
    {
        self.keep_if(move |v| !f(v))
    }
    /// Will start with the same value, but then only values matching the provided closures will be *kept*.
    #[track_caller]
    fn keep_if(&self, mut f: impl FnMut(&Self::Inner) -> bool + 'static) -> Signal<Self::Inner>
    where
        Self::Inner: Clone,
    {
        let ret = create_rw_signal(self.with_untracked(Clone::clone));
        self.for_each(move |value| {
            if f(value) {
                ret.set(value.clone());
            }
        });
        ret.into()
    }

    #[track_caller]
    fn not(&self) -> Signal<<Self::Inner as Not>::Output>
    where
        Self::Inner: Clone + Not,
    {
        self.map(|v| v.clone().not())
    }
    #[track_caller]
    fn is(&self, target: Self::Inner) -> Signal<bool>
    where
        Self::Inner: Eq,
    {
        self.map(move |v| v == &target)
    }

    /// Executes the provided closure over each Inner of the signal, *including* the current one.
    #[track_caller]
    fn for_each(&self, f: impl FnMut(&Self::Inner) + 'static) {
        let self_: Self = self.clone();
        let f = RefCell::new(f);
        create_render_effect(move |_| {
            self_.with(|value| untrack(|| f.borrow_mut()(value)));
        });
    }
    /// Executes the provided closure over each Inner of the signal, *excluding* the current one.
    #[track_caller]
    fn for_each_after_first(&self, mut f: impl FnMut(&Self::Inner) + 'static) {
        let mut first = true;
        self.for_each(move |value| {
            if first {
                first = false;
            } else {
                f(value);
            }
        });
    }
    /// Runs a function when the signal changes, taking the old and new Inner as arguments
    #[track_caller]
    fn for_each_window(&self, mut f: impl FnMut(&Self::Inner, &Self::Inner) + 'static)
    where
        Self::Inner: Clone,
    {
        let mut old = self.with_untracked(Clone::clone);
        self.for_each_after_first(move |new| {
            untrack(|| f(&old, new));
            old = new.clone();
        });
    }
}
impl<T, Value> ReadSignalExt for T
where
    T: SignalWith<Value = Value> + SignalWithUntracked<Value = Value> + Clone + 'static,
{
    type Inner = Value;
}

pub trait WriteSignalExt:
    ReadSignalExt
    + SignalSet<Value = <Self as ReadSignalExt>::Inner>
    + SignalUpdate<Value = <Self as ReadSignalExt>::Inner>
{
    #[track_caller]
    fn trigger_subscribers(&self) {
        self.update(|_| {}); // Current docs say this always triggers.
    }

    #[track_caller]
    fn set_if_changed(&self, value: Self::Inner)
    where
        Self::Inner: PartialEq,
    {
        if self.with_untracked(|old| old != &value) {
            self.set(value);
        }
    }
    /// Update the provided value in-place, and trigger the subscribers only if any edit has been done.
    fn update_if_changed(&self, f: impl FnOnce(&mut Self::Inner))
    where
        Self::Inner: PartialEq + Clone,
    {
        let mut new = self.with_untracked(Clone::clone);
        f(&mut new);
        self.set_if_changed(new);
    }

    #[track_caller]
    fn flip(&self)
    where
        Self::Inner: Clone + Not<Output = Self::Inner>,
    {
        self.set(self.with_untracked(Clone::clone).not());
    }
    #[track_caller]
    fn modify(&self) -> Modify<Self>
    where
        Self::Inner: Clone,
    {
        Modify {
            value: Some(self.with_untracked(Clone::clone)),
            signal: self.clone(),
        }
    }

    // TODO: get rid of this by adding derived rw signals? Slices?
    // Here it would be useful to have the rw equivalent of [Signal].
    fn double_bind<U>(
        self,
        mut from: impl FnMut(&Self::Inner) -> U + 'static,
        mut to: impl FnMut(&U) -> Self::Inner + 'static,
    ) -> RwSignal<U>
    where
        U: Clone,
    {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        enum Status {
            Idle,
            ReactingParent,
            ReactingChild,
        }

        let child: RwSignal<U> = create_rw_signal(self.with_untracked(&mut from));

        let lock = SharedBox::new(Status::Idle);

        self.for_each_after_first({
            let lock = lock.clone();
            move |value| match lock.get() {
                Status::Idle => {
                    lock.from_to(&Status::Idle, Status::ReactingParent);
                    child.set(from(value));
                    lock.from_to(&Status::ReactingParent, Status::Idle);
                }
                Status::ReactingParent => unreachable!(),
                Status::ReactingChild => {}
            }
        });

        let self_ = self.clone();
        child.for_each_after_first(move |value| match lock.get() {
            Status::Idle => {
                lock.from_to(&Status::Idle, Status::ReactingChild);
                self_.set(to(value));
                lock.from_to(&Status::ReactingChild, Status::Idle);
            }
            Status::ReactingParent => {}
            Status::ReactingChild => unreachable!(),
        });

        child
    }
}
impl<T, Value> WriteSignalExt for T where
    T: ReadSignalExt<Inner = Value>
        + SignalSet<Value = Value>
        + SignalUpdate<Value = Value>
        + Clone
{
}

pub struct Modify<T: WriteSignalExt> {
    value: Option<<T as ReadSignalExt>::Inner>,
    signal: T,
}
impl<T> fmt::Debug for Modify<T>
where
    T: WriteSignalExt + fmt::Debug,
    <T as ReadSignalExt>::Inner: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Modify")
            .field("value", &self.value)
            .field("signal", &self.signal)
            .finish()
    }
}
impl<T: WriteSignalExt> Deref for Modify<T> {
    type Target = <T as ReadSignalExt>::Inner;

    fn deref(&self) -> &Self::Target {
        self.value.as_ref().unwrap()
    }
}
impl<T: WriteSignalExt> DerefMut for Modify<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value.as_mut().unwrap()
    }
}
impl<T: WriteSignalExt> Drop for Modify<T> {
    fn drop(&mut self) {
        self.signal.set(self.value.take().unwrap());
    }
}

/// Useful to handle an aggregation over a variable (increasing for now) number of signals.
#[derive(Default)]
pub struct SignalBag<I> {
    trigger: RwSignal<()>,
    bag: Rc<RefCell<Vec<Getter<I>>>>,
}
type Getter<I> = Box<dyn Fn() -> I + 'static>;
impl<I: Clone + 'static> SignalBag<I> {
    pub fn new() -> Self {
        Self {
            trigger: create_rw_signal(()),
            bag: Rc::default(),
        }
    }
    pub fn push(&self, signal: impl ReadSignalExt<Inner = I> + 'static) {
        // We make sure future changes trigger an update.
        let trigger = self.trigger;
        signal.for_each_after_first(move |_| trigger.trigger_subscribers());

        self.bag
            .borrow_mut()
            .push(Box::new(move || signal.with(Clone::clone)));
        self.trigger.trigger_subscribers();
    }
    pub fn map<O: 'static>(&self, mut f: impl FnMut(Vec<I>) -> O + 'static) -> Signal<O> {
        let bag = self.bag.clone();
        self.trigger.map(move |&()| {
            let inputs: Vec<_> = bag.borrow().iter().map(|f| f()).collect();
            f(inputs)
        })
    }
}
impl<I: fmt::Debug> fmt::Debug for SignalBag<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignalBag")
            .field("trigger", &self.trigger)
            .field("bag_size", &self.bag.borrow().len())
            .finish()
    }
}
impl<I> Clone for SignalBag<I> {
    fn clone(&self) -> Self {
        Self {
            trigger: self.trigger,
            bag: self.bag.clone(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Load<T> {
    Loading,
    Ready(T),
}
