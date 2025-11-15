// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;

use crate::core::{MessageContext, Mut, ViewMarker};
use crate::{MessageResult, Pod, View, ViewCtx, WidgetView};

use masonry::widgets;
use xilem_core::ViewPathTracker;

/// An element which holds radio buttons.
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
pub fn radio_group<State, Action, V, Value>(
    access_value: AccessV<State, Value>,
    child: V,
) -> RadioGroup<State, Action, V, Value>
where
    Value: 'static,
    State: 'static,
    Action: 'static,
    // F: Fn(&mut State) -> &mut Value + 'static + Send + Sync + Clone + PartialEq,
    V: WidgetView<State, Action>,
{
    RadioGroup {
        child,
        access_value,
        phantom: PhantomData,
    }
}

type AccessV<State, Value> = fn(&mut State) -> &mut Value;

/// The [`View`] created by [`radio_group`] from a bool value and a callback.
///
/// See `radio_group` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct RadioGroup<State, Action, V, Value> {
    child: V,
    access_value: AccessV<State, Value>,
    phantom: PhantomData<fn() -> Action>,
}

pub(crate) struct RadioGroupState<State, Value> {
    pub(crate) access_value: AccessV<State, Value>,
}

impl<State: 'static, Value: 'static> std::fmt::Debug for RadioGroupState<State, Value> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RadioGroupState")
            .field(
                "access_value",
                &format!(
                    "(&mut {}) -> &mut {}",
                    std::any::type_name::<State>(),
                    std::any::type_name::<Value>()
                ),
            )
            .finish()
    }
}

impl<State: 'static, Value: 'static> xilem_core::Resource for RadioGroupState<State, Value> {}

impl<State, Action, V, Value> ViewMarker for RadioGroup<State, Action, V, Value> {}
impl<State, Action, V, Value> View<State, Action, ViewCtx> for RadioGroup<State, Action, V, Value>
where
    Value: 'static + Clone,
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = Pod<widgets::RadioGroup>;
    type ViewState = xilem_core::ProvidesState<V::ViewState>;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        // Prepare the initial state value
        let environment_item = xilem_core::EnvironmentItem {
            change_listeners: Vec::new(),
            value: Box::new(RadioGroupState {
                access_value: self.access_value,
            }),
        };

        let env = ctx.environment();
        let pos = env.create_slot_for_type::<RadioGroupState<State, Value>>();

        // Run the child build with the context value we're providing in the slot.
        let slot_idx = usize::try_from(pos).unwrap();
        let slot = &mut env.slots[slot_idx];
        slot.ref_count += 1;
        let old_value = slot.item.replace(environment_item);
        #[cfg(debug_assertions)]
        if let Some(old_value) = old_value.as_ref() {
            assert!(
                old_value.value.is::<RadioGroupState<State, Value>>(),
                "In providing {}, the type of the old value didn't match. The old value was instead {:?}",
                core::any::type_name::<RadioGroupState<State, Value>>(),
                old_value.value
            );
        }
        let (child_element, child_state) = self.child.build(ctx, app_state);
        let widget = widgets::RadioGroup::new(child_element.new_widget);

        // Restore the prior value into the environment
        let env = ctx.environment();
        let slot = &mut env.slots[slot_idx];
        let my_item = core::mem::replace(&mut slot.item, old_value);
        let my_item =
            my_item.expect("Child Views should not have deleted the environment item's value.");
        debug_assert!(
            my_item.value.is::<RadioGroupState<State, Value>>(),
            "Running a child build should have restored the same value"
        );

        let state = xilem_core::ProvidesState {
            child_state,
            this_state: Some(my_item),
            environment_slot: pos,
        };
        (ctx.create_pod(widget), state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        // TODO: Should the new value be updated by accessing the value with the previous function?
        if !std::ptr::fn_addr_eq(prev.access_value, self.access_value) {
            *(self.access_value)(app_state) = (prev.access_value)(app_state).clone();
        }
        let env = ctx.environment();
        let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
        debug_assert!(
            view_state.this_state.is_some(),
            "`Provides` should be providing something."
        );
        core::mem::swap(&mut slot.item, &mut view_state.this_state);

        let mut child = widgets::RadioGroup::child_mut(&mut element)
            .expect("We only create RadioGroup with a child");
        self.child.rebuild(
            &prev.child,
            &mut view_state.child_state,
            ctx,
            child.downcast(),
            app_state,
        );

        let env = ctx.environment();
        let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
        core::mem::swap(&mut slot.item, &mut view_state.this_state);
        debug_assert!(
            view_state.this_state.is_some(),
            "`Provides` should get its value back."
        );
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        // Make our value available in the child teardown.
        let env = ctx.environment();
        let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
        core::mem::swap(&mut slot.item, &mut view_state.this_state);

        let mut child = widgets::RadioGroup::child_mut(&mut element)
            .expect("We only create RadioGroup with a child");
        self.child
            .teardown(&mut view_state.child_state, ctx, child.downcast());

        let env = ctx.environment();
        let slot = &mut env.slots[usize::try_from(view_state.environment_slot).unwrap()];
        core::mem::swap(&mut slot.item, &mut view_state.this_state);
        slot.ref_count -= 1;
        if slot.ref_count == 0 {
            assert!(
                slot.item.is_none(),
                "Ref count for {slot:?} was not properly managed."
            );
            env.free_slots.push(view_state.environment_slot);
            env.types
                .remove(&std::any::TypeId::of::<RadioGroupState<State, Value>>());
        }
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        // Use our value in the child message.
        let slot =
            &mut message.environment.slots[usize::try_from(view_state.environment_slot).unwrap()];
        debug_assert!(
            view_state.this_state.is_some(),
            "`Provides` should be providing something."
        );
        core::mem::swap(&mut slot.item, &mut view_state.this_state);

        // TODO: Any need for a message directly to this view?
        // TODO: When the context/environment is available in messages, add the context value here.
        let mut child = widgets::RadioGroup::child_mut(&mut element)
            .expect("We only create RadioGroup with a child");
        let ret = self.child.message(
            &mut view_state.child_state,
            message,
            child.downcast(),
            app_state,
        );

        let slot =
            &mut message.environment.slots[usize::try_from(view_state.environment_slot).unwrap()];
        core::mem::swap(&mut slot.item, &mut view_state.this_state);
        debug_assert!(
            view_state.this_state.is_some(),
            "`Provides` should get its value back."
        );
        ret
    }
}
