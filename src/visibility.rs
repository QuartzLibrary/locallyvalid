use leptos::window;
use std::ops::Range;
use web_sys::{DomRect, Element};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Visibility {
    /// The box is before the viewport and not visible.
    Before,
    /// The box is straddling the beginning of the viewport, and its latter edge is visible.
    PeekingBefore(f64),
    /// The box is fully inside the viewport.
    Inside,
    /// The box is straddling the end of the viewport, and its first edge is visible.
    PeekingAfter(f64),
    /// The box is after the viewport and not visible.
    After,
    /// The box is straddling the viewport, with both edges off screen.
    Straddling(f64),
}
impl Visibility {
    pub fn vertical_from_element(element: impl AsRef<Element>, view: &ViewportSize) -> Self {
        Self::vertical_from_rect(&element.as_ref().get_bounding_client_rect(), view)
    }
    pub fn vertical_from_rect(rect: &DomRect, view: &ViewportSize) -> Self {
        Self::new(rect.top()..rect.bottom(), view.height)
    }

    pub fn horizontal_from_element(element: impl AsRef<Element>, view: &ViewportSize) -> Self {
        Self::horizontal_from_rect(&element.as_ref().get_bounding_client_rect(), view)
    }
    pub fn horizontal_from_rect(rect: &DomRect, view: &ViewportSize) -> Self {
        Self::new(rect.left()..rect.right(), view.width)
    }

    fn new(range: Range<f64>, window: f64) -> Self {
        let Range { start, end } = range;

        match f64::total_cmp(&start, &0.) {
            std::cmp::Ordering::Less => {
                if end <= 0. {
                    // ||
                    // |  |
                    //    [   ]
                    //    0   w
                    Self::Before
                } else if end <= window {
                    // |    |
                    // |      |
                    //    [   ]
                    //    0   w
                    Self::PeekingBefore(end / (end - start))
                } else {
                    // |        |
                    //    [   ]
                    //    0   w
                    Self::Straddling(window / (end - start))
                }
            }
            std::cmp::Ordering::Equal => {
                if end < 0. {
                    unreachable!()
                } else if end == 0. {
                    //    |
                    //    [   ]
                    //    0   w
                    Self::Before
                } else if end <= window {
                    //    | |
                    //    [   ]
                    //    0   w
                    Self::Inside
                } else {
                    //    |     |
                    //    [   ]
                    //    0   w
                    Self::PeekingAfter(window / (end - start))
                }
            }
            std::cmp::Ordering::Greater if start < window => {
                if end <= 0. {
                    unreachable!()
                } else if end <= window {
                    //     | |
                    //     |  |
                    //    [   ]
                    Self::Inside
                } else {
                    //      |    |
                    //    [   ]
                    //    0   w
                    Self::PeekingAfter((window - start) / (end - start))
                }
            }
            std::cmp::Ordering::Greater => {
                //        |
                //        |  |
                //    [   ]
                Self::After
            }
        }
    }
    pub fn fraction_visible(self) -> Option<f64> {
        match self {
            Visibility::Before | Visibility::After => None,

            Visibility::PeekingBefore(f)
            | Visibility::PeekingAfter(f)
            | Visibility::Straddling(f) => Some(f),

            Visibility::Inside => Some(1.),
        }
    }
    pub fn is_visible(self) -> bool {
        self.fraction_visible().is_some()
    }
    pub fn is_first_visible(self) -> bool {
        match self {
            Visibility::PeekingBefore(_) | Visibility::Straddling(_) => true,

            Visibility::Before
            | Visibility::Inside
            | Visibility::PeekingAfter(_)
            | Visibility::After => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportSize {
    width: f64,
    height: f64,
}
impl ViewportSize {
    pub fn from_global() -> Self {
        let window = window();
        Self {
            width: window.inner_width().unwrap().as_f64().unwrap(),
            height: window.inner_height().unwrap().as_f64().unwrap(),
        }
    }
}
