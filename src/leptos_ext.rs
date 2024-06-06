use leptos::{
    create_memo, create_render_effect, create_rw_signal, untrack, RwSignal, Signal, SignalSet,
    SignalUpdate, SignalWith, SignalWithUntracked,
};
use std::{
    cell::RefCell,
    fmt,
    ops::{Deref, DerefMut, Not},
    rc::Rc,
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

pub mod rc_signal {
    use leptos::{
        create_rw_signal, store_value, Owner, RwSignal, Signal, SignalDispose, SignalGet,
        SignalGetUntracked, SignalSet, SignalSetUntracked, SignalUpdate, SignalUpdateUntracked,
        SignalWith, SignalWithUntracked,
    };
    use std::{
        hash::{Hash, Hasher},
        ops::Deref,
        rc::Rc,
    };

    /// To suppress warning when leaking complex global signals. Use with caution on shallow function.
    pub fn with_intentional_leak<Out>(f: impl FnOnce() -> Out) -> Out {
        // TODO: supress warning
        leptos::with_owner(Owner::from_ffi(u64::MAX), f)
    }

    /// A leaked [RwSignal].
    /// It is important that there are no ways to construct this without leaking it, including during deserialisation.
    ///
    /// This is equivalent to a [RwSignal], but it is never disposed of unless done manually by the user
    /// or the runtime itself is disposed of.
    ///
    /// Useful to statically guarantee a global signal is not disposed of accidentally.
    #[derive(Debug, PartialEq, Eq)]
    pub struct LeakedRwSignal<T: 'static>(RwSignal<T>);
    impl<T: 'static> LeakedRwSignal<T> {
        /// Creates a new [LeakedRwSignal], see type docs for more.
        #[inline(always)]
        #[track_caller]
        pub fn new(value: T) -> Self {
            with_intentional_leak(|| Self(create_rw_signal(value)))
        }
    }
    impl<T: 'static> Copy for LeakedRwSignal<T> {}
    impl<T: 'static> Clone for LeakedRwSignal<T> {
        #[allow(clippy::non_canonical_clone_impl)] // We don't need the T: Clone bound.
        fn clone(&self) -> Self {
            Self(self.0)
        }
    }
    impl<T: Hash + 'static> Hash for LeakedRwSignal<T> {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.with(|v| v.hash(state));
        }
    }
    impl<T: 'static> Deref for LeakedRwSignal<T> {
        type Target = RwSignal<T>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<T: Default + 'static> Default for LeakedRwSignal<T> {
        fn default() -> Self {
            Self::new(T::default())
        }
    }
    impl<T: 'static> From<LeakedRwSignal<T>> for Signal<T> {
        fn from(value: LeakedRwSignal<T>) -> Self {
            (*value).into()
        }
    }

    /// A version of [RwSignal] that handles cleanup using an [Rc].
    ///
    /// This is useful when using nested signals, since keeping track of scopes can be bothersome then.
    #[derive(Debug, PartialEq, Eq, Default)]
    pub struct RcSignal<T: 'static> {
        inner: Rc<RcSignalInner<T>>,
    }
    #[derive(Debug, PartialEq, Eq, Default)]
    struct RcSignalInner<T: 'static>(LeakedRwSignal<T>);
    impl<T: 'static> Drop for RcSignalInner<T> {
        fn drop(&mut self) {
            self.0 .0.dispose();
        }
    }
    impl<T: 'static> RcSignal<T> {
        /// Creates a new [RcSignal], see type docs for more.
        #[inline(always)]
        #[track_caller]
        pub fn new(value: T) -> Self {
            Self {
                inner: Rc::new(RcSignalInner(LeakedRwSignal::new(value))),
            }
        }
        /// The returned [RwSignal] will live at least as long as a signal created here.
        ///
        /// (That is, it'll live as long as a signal owned by the current [Owner](crate::Owner).)
        pub fn into_rw(&self) -> RwSignal<T> {
            // We store the Rc, ensuring that the signal is kept at least as long as a signal that would be created here.
            let _ = store_value(self.clone());
            self.inner.0 .0
        }
    }
    impl<T: 'static> Clone for RcSignal<T> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
            }
        }
    }
    impl<T: Hash + 'static> Hash for RcSignal<T> {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.inner.0.hash(state);
        }
    }
    impl<T: 'static> SignalWithUntracked for RcSignal<T> {
        type Value = T;

        fn with_untracked<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> O {
            self.inner.0.with_untracked(f)
        }

        fn try_with_untracked<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> Option<O> {
            self.inner.0.try_with_untracked(f)
        }
    }
    impl<T: 'static> SignalWith for RcSignal<T> {
        type Value = T;

        fn with<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> O {
            self.inner.0.with(f)
        }

        fn try_with<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> Option<O> {
            self.inner.0.try_with(f)
        }
    }
    impl<T: Clone + 'static> SignalGetUntracked for RcSignal<T> {
        type Value = T;

        fn get_untracked(&self) -> Self::Value {
            self.inner.0.get_untracked()
        }

        fn try_get_untracked(&self) -> Option<Self::Value> {
            self.inner.0.try_get_untracked()
        }
    }
    impl<T: Clone + 'static> SignalGet for RcSignal<T> {
        type Value = T;

        fn get(&self) -> Self::Value {
            self.inner.0.get()
        }

        fn try_get(&self) -> Option<Self::Value> {
            self.inner.0.try_get()
        }
    }
    impl<T: 'static> SignalSet for RcSignal<T> {
        type Value = T;

        fn set(&self, new_value: T) {
            self.inner.0.set(new_value);
        }

        fn try_set(&self, new_value: T) -> Option<T> {
            self.inner.0.try_set(new_value)
        }
    }
    impl<T: 'static> SignalSetUntracked<T> for RcSignal<T> {
        fn set_untracked(&self, new_value: T) {
            self.inner.0.set_untracked(new_value);
        }

        fn try_set_untracked(&self, new_value: T) -> Option<T> {
            self.inner.0.try_set_untracked(new_value)
        }
    }
    impl<T: Clone + 'static> SignalUpdate for RcSignal<T> {
        type Value = T;

        fn update(&self, f: impl FnOnce(&mut Self::Value)) {
            self.inner.0.update(f);
        }

        fn try_update<O>(&self, f: impl FnOnce(&mut Self::Value) -> O) -> Option<O> {
            self.inner.0.try_update(f)
        }
    }
    impl<T: Clone + 'static> SignalUpdateUntracked<T> for RcSignal<T> {
        fn update_untracked(&self, f: impl FnOnce(&mut T)) {
            self.inner.0.update_untracked(f);
        }

        fn try_update_untracked<O>(&self, f: impl FnOnce(&mut T) -> O) -> Option<O> {
            self.inner.0.try_update_untracked(f)
        }
    }
}
