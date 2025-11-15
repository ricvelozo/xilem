// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::core::{MessageContext, Mut, ViewMarker};
use crate::view::RadioGroupState;
use crate::{MessageResult, Pod, View, ViewCtx};

use masonry::core::ArcStr;
use masonry::widgets::{self, RadioButtonToggled};
use xilem_core::ViewPathTracker as _;

/// An element which can be in checked and unchecked state.
///
/// # Example
/// ```
/// use xilem::view::{flex_row, radio_button, radio_group};
/// # use xilem::WidgetView;
///
/// #[derive(PartialEq, Clone)]
/// enum Fruit {
///     Banana,
///     Apple,
///     Lime,
/// }
///
/// struct State {
///     fruit: Fruit,
/// }
///
/// # fn view() -> impl WidgetView<State> {
/// radio_group(
///    |state: &mut State| &mut state.fruit,
///    flex_row((
///        radio_button("Banana", Fruit::Banana),
///        radio_button("Apple", Fruit::Apple),
///        radio_button("Lime", Fruit::Lime),
///     ))
/// )
/// # }
/// ```
pub fn radio_button<Value>(label: impl Into<ArcStr>, value: Value) -> RadioButton<Value>
where
    Value: PartialEq + Clone + 'static,
{
    RadioButton {
        label: label.into(),
        value,
        disabled: false,
    }
}

/// The [`View`] created by [`radio_button`] from a `label`, a bool value and a callback.
///
/// See `radio_button` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct RadioButton<Value> {
    label: ArcStr,
    value: Value,
    disabled: bool,
}

impl<Value> RadioButton<Value> {
    /// Set the disabled state of the widget.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<Value> ViewMarker for RadioButton<Value> {}
impl<Value, State, Action> View<State, Action, ViewCtx> for RadioButton<Value>
where
    State: 'static,
    Value: PartialEq + Clone + 'static,
{
    type Element = Pod<widgets::RadioButton>;
    type ViewState = xilem_core::WithContextState<(), ()>;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let path: std::sync::Arc<[xilem_core::ViewId]> = ctx.view_path().into();
        // ctx.with_action_widget(|ctx| {
        let env = ctx.environment();
        let pos = env.get_slot_for_type::<RadioGroupState<State, Value>>();
        let Some(pos) = pos else {
            // TODO: panic? Or warning, that no radio group with this `Value` was provided?
            panic!(
                // TODO: Track caller for this view?
                "Xilem: Tried to get `radio_group` value access function for {}, \
                    but it hasn't been provided. \
                    Did you forget to wrap this view with a `xilem::view::radio_group`?",
                core::any::type_name::<Value>()
            );
        };
        let slot_idx = usize::try_from(pos).unwrap();
        let slot = &mut env.slots[slot_idx];
        // TODO: Should this be &mut or just a shared ref?
        // If this gets modified, we won't rerun any other WithContexts for this value
        // But some types are "pure", i.e. they manage their own dependencies?
        let Some(value) = slot.item.as_mut() else {
            panic!(
                // TODO: Track caller for this view?
                "Xilem: Tried to get context for {}, but it hasn't been `Provided`.",
                core::any::type_name::<RadioGroupState<State, Value>>()
            );
        };
        let access_value = value
            .value
            .downcast_mut::<RadioGroupState<State, Value>>()
            .expect("Environment's slots should have the correct types.");

        let mut first_empty = None;
        let mut needs_storing = true;
        // We store the path to this reader as a listener.
        // This is required so that we can be alerted of any changes, so that any parent
        // memoizing (or similar) views would correctly handle our value changing.
        // N.B. This is strictly only needed if:
        // 1) There actually is such a parent view
        // 2) The path for rebuilding only needs to be the path to the closest such parent
        //
        // The future changes required to enable that are already partially accounted for here (i.e. checking
        // if the current listening path is already included).
        //
        // Note also that there is currently no way to trigger these views!
        for (idx, item) in value.change_listeners.iter().enumerate() {
            if let Some(item) = item {
                if **item == *path {
                    needs_storing = false;
                    break;
                }
            } else {
                first_empty.get_or_insert(idx);
            }
        }

        let listener_index = if needs_storing {
            if let Some(first_empty) = first_empty {
                value.change_listeners[first_empty] = Some(path);
                Some(first_empty)
            } else {
                let idx = value.change_listeners.len();
                value.change_listeners.push(Some(path));
                Some(idx)
            }
        } else {
            None
        };
        let checked = *(access_value.access_value)(app_state) == self.value;

        let mut pod = ctx.create_pod(widgets::RadioButton::new(checked, self.label.clone()));
        pod.new_widget.options.disabled = self.disabled;
        let state = xilem_core::WithContextState {
            prev: (),
            child_state: (),
            environment_slot: pos,
            listener_index,
        };
        ctx.record_action(pod.new_widget.id());

        (pod, state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        if prev.disabled != self.disabled {
            element.ctx.set_disabled(self.disabled);
        }
        if prev.label != self.label {
            widgets::RadioButton::set_text(&mut element, self.label.clone());
        }
        let env = ctx.environment();
        let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
        let Some(value) = slot.item.as_mut() else {
            panic!(
                // TODO: Track caller for this view?
                "Xilem: Tried to get access_value for {}, but it hasn't been provided by a radio group",
                core::any::type_name::<RadioGroupState<State, Value>>()
            );
        };
        let access_value = value
            .value
            .downcast_mut::<RadioGroupState<State, Value>>()
            .expect("Environment's slots should have the correct types.");
        let checked = *(access_value.access_value)(app_state) == self.value;
        widgets::RadioButton::set_checked(&mut element, checked);
    }

    fn teardown(
        &self,
        _: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        debug_assert!(
            message.remaining_path().is_empty(),
            "id path should be empty in RadioButton::message"
        );
        match message.take_message::<RadioButtonToggled>() {
            Some(_) => {
                let env = &mut message.environment;
                let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
                let Some(value) = slot.item.as_mut() else {
                    panic!(
                        // TODO: Track caller for this view?
                        "Xilem: Tried to get access_value for {}, but it hasn't been provided by a radio group",
                        core::any::type_name::<RadioGroupState<State, Value>>()
                    );
                };
                let access_value = value
                    .value
                    .downcast_mut::<RadioGroupState<State, Value>>()
                    .expect("Environment's slots should have the correct types.");
                let was_checked = *(access_value.access_value)(app_state) == self.value;
                if !was_checked {
                    *(access_value.access_value)(app_state) = self.value.clone();
                    widgets::RadioButton::set_checked(&mut element, true);
                    MessageResult::RequestRebuild
                } else {
                    MessageResult::Nop
                }
            }
            None => {
                tracing::error!("Wrong message type in RadioButton::message, got {message:?}.");
                MessageResult::Stale
            }
        }
    }
}
