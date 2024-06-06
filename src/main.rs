// pub mod human;
pub mod leptos_ext;
pub mod visibility;

use chrono::{DateTime, Utc};
use leptos::{
    create_memo, create_render_effect, document, ev, html, html::ToHtmlElement, mount_to_body,
    on_cleanup, window_event_listener, CollectView, HtmlElement, IntoView, RwSignal, Signal,
    SignalGet, SignalGetUntracked, SignalSet, SignalWith, SignalWithUntracked, View,
};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    ops::{Bound, Deref},
    rc::Rc,
};
use web_sys::{wasm_bindgen::JsCast, HtmlDivElement, Node};

use self::{
    leptos_ext::{ReadSignalExt, WriteSignalExt},
    visibility::{ViewportSize, Visibility},
};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
struct Data {
    entries: BTreeMap<u128, Entry>,
    children: BTreeMap<u128, BTreeSet<u128>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Entry {
    text: String,
    parents: Vec<u128>,
}

pub fn main() {
    console_log::init().unwrap();

    log::info!("Init");

    mount_to_body(app);
}

fn app() -> impl IntoView {
    let current = RwSignal::new(999);
    let data = RwSignal::new(initial_data());

    html::div().class("graph", true).child(graph(current, data))
}
fn graph(current: RwSignal<u128>, data: RwSignal<Data>) -> impl IntoView {
    move || {
        let initial = current.get();
        let data = data.get();

        match data.entries.get(&initial) {
            Some(entry) => [
                html::div()
                    .style("width", "100%")
                    .style("height", "40px")
                    .into_view(),
                graph_upstream(initial, data.clone(), BTreeSet::new()).into_view(),
                card(initial, entry).class("current", true).into_view(),
                graph_downstream(initial, data, BTreeSet::new()).into_view(),
                explanation().into_view(),
                html::div()
                    .style("width", "100%")
                    .style("height", "150vh")
                    .into_view(),
            ]
            .into_view(),
            None => empty_card(initial, "No initial value").into_view(),
        }
    }
}
fn graph_upstream(child: u128, data: Data, mut done: BTreeSet<u128>) -> impl IntoView {
    let Some(entry) = data.entries.get(&child).cloned() else {
        return "Missing entry".into_view();
    };

    if done.contains(&child) {
        return "Repeated".into_view();
    }
    done.insert(child);

    let Some(first) = entry.parents.first().cloned() else {
        return View::default();
    };
    let current_parent = RwSignal::new(first);

    let parent_ids: Vec<_> = entry.parents.clone();
    let parents: Vec<_> = entry
        .parents
        .clone()
        .into_iter()
        .map(|p| match data.entries.get(&p) {
            Some(entry) => card(p, entry).class("current", move || current_parent.get() == p),
            None => empty_card(p, "Missing parent"),
        })
        .collect();

    let is_single = parents.len() == 1;

    let spacer = RwSignal::new(0.);

    [
        html::div()
            .style("width", "100%")
            .style("height", spacer.map_dedup(|v| format!("{v}px")))
            .into_view(),
        {
            let data = data.clone();
            move || {
                let current_parent = current_parent.get();
                graph_upstream(current_parent, data.clone(), done.clone())
            }
        }
        .into_view(),
        html::div()
            .class("row", true)
            .class("single", is_single)
            .on(ev::scroll, {
                let parents = parents.clone();
                let parent_ids = parent_ids.clone();
                let frame = Rc::new(RefCell::new(None));
                move |_| {
                    let parents = parents.clone();
                    let parent_ids = parent_ids.clone();

                    let inner = frame.clone();
                    let new = frame.take().unwrap_or_else(move || {
                        gloo_render::request_animation_frame(move |_| {
                            let (first_id, first_e) = first_visible_element(&parent_ids, &parents);

                            if current_parent.get_untracked() != first_id {
                                let top = first_e.get_bounding_client_rect().top();
                                current_parent.set(first_id);
                                restore_position(top, first_e, spacer);
                            }

                            drop(inner.take());
                        })
                    });
                    frame.replace(Some(new));
                }
            })
            .child(parents)
            .into_view(),
    ]
    .into_view()
}
fn graph_downstream(parent: u128, data: Data, mut done: BTreeSet<u128>) -> impl IntoView {
    if done.contains(&parent) {
        return "Repeated".into_view();
    }
    done.insert(parent);

    let Some(_) = data.entries.get(&parent).cloned() else {
        return "Missing entry".into_view();
    };

    let child_ids: Vec<_> = data
        .children
        .get(&parent)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect();

    let Some(first) = child_ids.first().cloned() else {
        return View::default();
    };
    let current_child = RwSignal::new(first);

    let children: Vec<_> = child_ids
        .clone()
        .into_iter()
        .map(|c| match data.entries.get(&c) {
            Some(entry) => card(c, entry).class("current", move || current_child.get() == c),
            None => empty_card(c, "Missing child"),
        })
        .collect();

    let is_single = child_ids.len() == 1;

    [
        html::div()
            .class("row", true)
            .class("single", is_single)
            .on(ev::scroll, {
                let children = children.clone();
                let child_ids = child_ids.clone();
                let frame = Rc::new(RefCell::new(None));
                move |_| {
                    let children = children.clone();
                    let child_ids = child_ids.clone();

                    let inner = frame.clone();
                    let new = frame.take().unwrap_or_else(move || {
                        gloo_render::request_animation_frame(move |_| {
                            let (first_id, first_e) = first_visible_element(&child_ids, &children);
                            current_child.set_if_changed(first_id);

                            drop(inner.take());
                        })
                    });
                    frame.replace(Some(new));
                }
            })
            .child(children)
            .into_view(),
        {
            let data = data.clone();
            move || {
                let current_child = current_child.get();
                graph_downstream(current_child, data.clone(), done.clone())
            }
        }
        .into_view(),
    ]
    .into_view()
}

fn first_visible_element(
    ids: &[u128],
    elements: &[HtmlElement<html::Div>],
) -> (u128, HtmlElement<html::Div>) {
    let view = ViewportSize::from_global();
    for (id, e) in ids.iter().zip(elements) {
        match Visibility::horizontal_from_element(e.deref(), &view) {
            Visibility::Before => {}
            Visibility::PeekingBefore(_) | Visibility::Inside => return (*id, e.clone()),
            Visibility::PeekingAfter(_) | Visibility::After | Visibility::Straddling(_) => {
                unreachable!()
            }
        }
    }

    unreachable!()
}

fn restore_position(at: f64, e: HtmlElement<html::Div>, spacer: RwSignal<f64>) {
    let window = leptos::window();

    spacer.set_if_changed(0.);

    let mut top = e.get_bounding_client_rect().top();
    let mut delta = top - at;
    let margin = delta + window.scroll_y().unwrap();

    log::warn!("top:{top} old_top:{at} delta:{delta} margin:{margin}");

    if margin <= 0. {
        spacer.set_if_changed(-margin);
        top = e.get_bounding_client_rect().top();
        delta = top - at;
    } else {
        // spacer.set_if_changed(0.);
    }
    window.scroll_to_with_x_and_y(0., delta);
}

fn card(id: u128, entry: &Entry) -> HtmlElement<html::Div> {
    html::div()
        .attr("card-id", id)
        .class("card", true)
        .child(entry.text.clone())
}
fn empty_card(id: u128, message: impl AsRef<str>) -> HtmlElement<html::Div> {
    let message = message.as_ref().to_owned();
    html::div()
        .attr("card-id", id)
        .class("card", true)
        .child(message)
}

fn explanation() -> impl IntoView {
    html::div()
    .class("explanation", true)
    .child(html::p().child("This is a small test to see if an idea I had for better interface for reference world models would work. The following example is picked from ")
    .child(html::a().attr("href", "https://www.lesswrong.com/posts/uMQ3cqWDPHhjtiesc/agi-ruin-a-list-of-lethalities")
    .child("AGI Ruin: A List of Lethalities"))
    .child(", mostly because it's already split into convenient points with bolded claims. I quickly wrote down some dependencies between claims to test, and haven't confirmed they fully make sense yet."))
    .child(html::p().child("To navigate just scroll sideways. Red cards are active."))
    .into_view()
}

impl Data {
    fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
    fn from_json(raw: &str) -> Result<Self, ()> {
        let entries: BTreeMap<u128, Entry> = serde_json::from_str(raw).map_err(drop)?;
        Ok(Self::from_raw(entries))
    }
    fn from_raw(entries: BTreeMap<u128, Entry>) -> Self {
        let mut children: BTreeMap<u128, BTreeSet<u128>> = BTreeMap::new();
        for (id, entry) in &entries {
            for p in &entry.parents {
                children.entry(*p).or_default().insert(*id);
            }
        }

        Self { entries, children }
    }
}

fn initial_data() -> Data {
    const INTIAL_DATA: &str = include_str!("./lol.json");
    Data::from_json(INTIAL_DATA).unwrap()
}
