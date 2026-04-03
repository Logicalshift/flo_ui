use super::button_state::*;
use super::focus_state::*;
use crate::subprograms::*;

use flo_scene::*;
use flo_scene::programs::*;
use flo_draw::*;

use futures::prelude::*;

use std::collections::{HashMap};

///
/// Runs the UI focus subprogram
///
pub async fn focus(input: InputStream<Focus>, context: SceneContext) {
    let program_id  = context.current_program_id().unwrap();
    let mut input   = input;

    // Request updates from the scene (which we'll use to remove subprograms that aren't running any more)
    context.send_message(SceneControl::Subscribe(program_id.into())).await.ok();

    // Create the state
    let mut focus = FocusProgram {
        canvas_program:             None,
        subprogram_space:           None,
        subprogram_data:            HashMap::new(),
        subprogram_order:           vec![],
        pointer_target:             None,
        pointer_target_program:     None,
        pointer_target_control:     None,
        button_state:               ButtonState::new(),
        focused_subprogram:         None,
        focused_control:            None,
        focused_event_target:       None,
        tab_ordering:               HashMap::new(),
        scale:                      None,
        bounds:                     None,
        hover:                      (None, None),
    };

    while let Some(request) = input.next().await {
        use Focus::*;

        match request {
            // General untargeted draw events
            Event(DrawEvent::Redraw)                => { },
            Event(DrawEvent::NewFrame)              => { },
            Event(DrawEvent::Scale(scale))          => { focus.set_scale(scale, &context).await; },
            Event(DrawEvent::Resize(w, h))          => { focus.set_bounds(w, h, &context).await; },
            Event(DrawEvent::CanvasTransform(_))    => { },
            Event(DrawEvent::Closed)                => { focus.send_to_all(FocusWindowEvent::Closed, &context).await; }

            // Pointer and key events
            Event(DrawEvent::Pointer(PointerAction::Enter, _, _))                           => { },
            Event(DrawEvent::Pointer(PointerAction::Leave, _, _))                           => { },
            Event(DrawEvent::Pointer(PointerAction::ButtonDown, pointer_id, pointer_state)) => { focus.set_pointer_target(&pointer_state, &context).await; focus.button_state.set_buttons(&pointer_state.buttons); focus.send_to_pointer_target(FocusPointerEvent::Pointer(None, PointerAction::ButtonDown, pointer_id, pointer_state)).await; },
            Event(DrawEvent::Pointer(PointerAction::ButtonUp, pointer_id, pointer_state))   => { focus.button_state.set_buttons(&pointer_state.buttons); focus.send_drop(&pointer_state).await; focus.send_to_pointer_target(FocusPointerEvent::Pointer(None, PointerAction::ButtonUp, pointer_id, pointer_state)).await; },
            Event(DrawEvent::Pointer(other_action, pointer_id, pointer_state))              => { focus.set_pointer_target(&pointer_state, &context).await; focus.send_hover(&pointer_state).await; focus.send_to_pointer_target(FocusPointerEvent::Pointer(None, other_action, pointer_id, pointer_state)).await; },
            Event(DrawEvent::KeyDown(scancode, key))                                        => { focus.send_to_focus(FocusKeyboardEvent::KeyDown(None, scancode, key), &context).await; },
            Event(DrawEvent::KeyUp(scancode, key))                                          => { focus.send_to_focus(FocusKeyboardEvent::KeyUp(None, scancode, key), &context).await; },

            // Updates from the scene in general
            Update(SceneUpdate::Stopped(program_id))    => { focus.remove_program_claims(program_id).await; focus.remove_program_focus(program_id).await; },
            Update(_)                                   => { }

            // Keyboard handling
            SetKeyboardFocus(program_id, control_id)                        => focus.set_keyboard_focus(program_id, control_id, &context).await,
            SetFollowingControl(program_id, control_id, next_control_id)    => focus.set_following_control(program_id, control_id, next_control_id).await,
            SetFollowingSubProgram(program_id, next_program_id)             => focus.set_following_subprogram(program_id, next_program_id).await,
            FocusNext                                                       => focus.focus_next(&context).await,
            FocusPrevious                                                   => focus.focus_previous(&context).await,

            // Control handling
            RemoveClaim(program_id)                                     => focus.remove_program_claims(program_id).await,
            RemoveControlClaim(program_id, control_id)                  => focus.remove_control_claims(program_id, control_id).await,
            SetCanvas(canvas_program_id)                                => focus.set_canvas(canvas_program_id).await,
            ClaimRegion { program, region, z_index }                    => focus.claim_region(program, region, None, z_index, &context).await,
            ClaimControlRegion { program, region, control, z_index }    => focus.claim_region(program, region, Some(control), z_index, &context).await,
        }
    }
}
