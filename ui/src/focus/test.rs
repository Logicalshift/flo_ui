use crate::subprograms::*;
use crate::util::*;

use flo_scene::*;
use flo_scene::programs::*;
use flo_draw::*;

use futures::prelude::*;

use std::result::{Result};
use serde::*;

fn expect_focus(evt: FocusEvent, control: ControlId, control_num: usize) -> Result<(), String> {
    if let FocusEvent::Keyboard(FocusKeyboardEvent::Focused(actual_control)) = evt {
        if actual_control == control {
            Ok(())
        } else {
            Err(format!("Expected focus control {}, got {:?}", control_num, evt))
        }
    } else {
        Err(format!("Expected focus control {}, got {:?}", control_num, evt))
    }
}

fn expect_unfocus(evt: FocusEvent, control: ControlId, control_num: usize) -> Result<(), String> {
    if let FocusEvent::Keyboard(FocusKeyboardEvent::Unfocused(actual_control)) = evt {
        if actual_control == control {
            Ok(())
        } else {
            Err(format!("Expected unfocus control {}, got {:?}", control_num, evt))
        }
    } else {
        Err(format!("Expected unfocus control {}, got {:?}", control_num, evt))
    }
}

#[test]
fn focus_following_control() {
    let test_program    = SubProgramId::called("focus_following_control");
    let scene           = Scene::default();

    let control_1       = ControlId::new();
    let control_2       = ControlId::new();
    let control_3       = ControlId::new();
    let control_4       = ControlId::new();

    println!("1 = {:?}, 2 = {:?}, 3 = {:?}, 4 = {:?}", control_1, control_2, control_3, control_4);

    TestBuilder::new()
        .send_message(Focus::SetFollowingControl(test_program, control_1, control_2))
        .send_message(Focus::SetFollowingControl(test_program, control_2, control_3))
        .send_message(Focus::SetFollowingControl(test_program, control_3, control_4))

        .send_message(Focus::SetKeyboardFocus(test_program, control_1))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_1, 1))
        .send_message(Focus::FocusNext)
        .expect_message(move |evt: FocusEvent| expect_unfocus(evt, control_1, 1))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_2, 2))
        .send_message(Focus::FocusNext)
        .expect_message(move |evt: FocusEvent| expect_unfocus(evt, control_2, 2))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_3, 3))
        .send_message(Focus::FocusNext)
        .expect_message(move |evt: FocusEvent| expect_unfocus(evt, control_3, 3))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_4, 4))
        .run_in_scene_with_threads(&scene, test_program, 5);
}

#[test]
fn focus_previous_control() {
    let test_program    = SubProgramId::called("focus_following_control");
    let scene           = Scene::default();

    let control_1       = ControlId::new();
    let control_2       = ControlId::new();
    let control_3       = ControlId::new();
    let control_4       = ControlId::new();

    println!("1 = {:?}, 2 = {:?}, 3 = {:?}, 4 = {:?}", control_1, control_2, control_3, control_4);

    TestBuilder::new()
        .send_message(Focus::SetFollowingControl(test_program, control_1, control_2))
        .send_message(Focus::SetFollowingControl(test_program, control_2, control_3))
        .send_message(Focus::SetFollowingControl(test_program, control_3, control_4))

        .send_message(Focus::SetKeyboardFocus(test_program, control_4))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_4, 4))
        .send_message(Focus::FocusPrevious)
        .expect_message(move |evt: FocusEvent| expect_unfocus(evt, control_4, 4))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_3, 3))
        .send_message(Focus::FocusPrevious)
        .expect_message(move |evt: FocusEvent| expect_unfocus(evt, control_3, 3))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_2, 2))
        .send_message(Focus::FocusPrevious)
        .expect_message(move |evt: FocusEvent| expect_unfocus(evt, control_2, 2))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_1, 1))
        .run_in_scene_with_threads(&scene, test_program, 5);
}

#[test]
fn focus_previous_control_makes_loop() {
    let test_program    = SubProgramId::called("focus_following_control");
    let scene           = Scene::default();

    let control_1       = ControlId::new();
    let control_2       = ControlId::new();
    let control_3       = ControlId::new();
    let control_4       = ControlId::new();

    println!("1 = {:?}, 2 = {:?}, 3 = {:?}, 4 = {:?}", control_1, control_2, control_3, control_4);

    TestBuilder::new()
        .send_message(Focus::SetFollowingControl(test_program, control_1, control_2))
        .send_message(Focus::SetFollowingControl(test_program, control_2, control_3))
        .send_message(Focus::SetFollowingControl(test_program, control_3, control_4))

        .send_message(Focus::SetKeyboardFocus(test_program, control_4))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_4, 4))
        .send_message(Focus::FocusPrevious)
        .expect_message(move |evt: FocusEvent| expect_unfocus(evt, control_4, 4))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_3, 3))
        .send_message(Focus::FocusPrevious)
        .expect_message(move |evt: FocusEvent| expect_unfocus(evt, control_3, 3))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_2, 2))
        .send_message(Focus::FocusPrevious)
        .expect_message(move |evt: FocusEvent| expect_unfocus(evt, control_2, 2))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_1, 1))
        .send_message(Focus::FocusPrevious)
        .expect_message(move |evt: FocusEvent| expect_unfocus(evt, control_1, 1))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_4, 4))
        .run_in_scene_with_threads(&scene, test_program, 5);
}

#[test]
fn focus_following_control_makes_loop() {
    let test_program    = SubProgramId::called("focus_following_control");
    let scene           = Scene::default();

    let control_1       = ControlId::new();
    let control_2       = ControlId::new();
    let control_3       = ControlId::new();
    let control_4       = ControlId::new();

    println!("1 = {:?}, 2 = {:?}, 3 = {:?}, 4 = {:?}", control_1, control_2, control_3, control_4);

    TestBuilder::new()
        .send_message(Focus::SetFollowingControl(test_program, control_1, control_2))
        .send_message(Focus::SetFollowingControl(test_program, control_2, control_3))
        .send_message(Focus::SetFollowingControl(test_program, control_3, control_4))

        .send_message(Focus::SetKeyboardFocus(test_program, control_1))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_1, 1))
        .send_message(Focus::FocusNext)
        .expect_message(move |evt: FocusEvent| expect_unfocus(evt, control_1, 1))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_2, 2))
        .send_message(Focus::FocusNext)
        .expect_message(move |evt: FocusEvent| expect_unfocus(evt, control_2, 2))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_3, 3))
        .send_message(Focus::FocusNext)
        .expect_message(move |evt: FocusEvent| expect_unfocus(evt, control_3, 3))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_4, 4))
        .send_message(Focus::FocusNext)
        .expect_message(move |evt: FocusEvent| expect_unfocus(evt, control_4, 4))
        .expect_message(move |evt: FocusEvent| expect_focus(evt, control_1, 1))
        .run_in_scene_with_threads(&scene, test_program, 5);
}

///
/// Checks that evt is an enter event for a subprogram and not a control
///
fn expect_enter_program(evt: FocusEvent) -> Result<(), String> {
    if let FocusEvent::Pointer(FocusPointerEvent::Pointer(control_id, PointerAction::Enter, _, _)) = evt {
        if control_id.is_none() {
            Ok(())
        } else {
            Err(format!("Expected PointerAction::Enter with a 'None' control ID, got {:?}", evt))
        }
    } else {
        Err(format!("Expected PointerAction::Enter, got {:?}", evt))
    }
}

///
/// Checks that evt is an enter event for a control
///
fn expect_enter_control(evt: FocusEvent, control_id: ControlId) -> Result<(), String> {
    if let FocusEvent::Pointer(FocusPointerEvent::Pointer(actual_control_id, PointerAction::Enter, _, _)) = evt {
        if actual_control_id == Some(control_id) {
            Ok(())
        } else {
            Err(format!("Expected PointerAction::Enter with a '{:?}' control ID, got {:?}", control_id, evt))
        }
    } else {
        Err(format!("Expected PointerAction::Enter, got {:?}", evt))
    }
}

///
/// Checks that evt is an enter event
///
fn expect_enter(evt: FocusEvent) -> Result<(), String> {
    if matches!(evt, FocusEvent::Pointer(FocusPointerEvent::Pointer(_, PointerAction::Enter, _, _))) {
        Ok(())
    } else {
        Err(format!("Expected PointerAction::Enter, got {:?}", evt))
    }
}

///
/// Checks that evt is an leave event
///
fn expect_leave(evt: FocusEvent) -> Result<(), String> {
    if matches!(evt, FocusEvent::Pointer(FocusPointerEvent::Pointer(_, PointerAction::Leave, _, _))) {
        Ok(())
    } else {
        Err(format!("Expected PointerAction::Leave, got {:?}", evt))
    }
}

///
/// Checks that evt is a button down event
///
fn expect_buttondown(evt: FocusEvent) -> Result<(), String> {
    if matches!(evt, FocusEvent::Pointer(FocusPointerEvent::Pointer(_, PointerAction::ButtonDown, _, _))) {
        Ok(())
    } else {
        Err(format!("Expected PointerAction::ButtonDown, got {:?}", evt))
    }
}

///
/// Checks that evt is a button up event
///
fn expect_buttonup(evt: FocusEvent) -> Result<(), String> {
    if matches!(evt, FocusEvent::Pointer(FocusPointerEvent::Pointer(_, PointerAction::ButtonUp, _, _))) {
        Ok(())
    } else {
        Err(format!("Expected PointerAction::ButtonUp, got {:?}", evt))
    }
}

///
/// Checks that evt is a move event
///
fn expect_move(evt: FocusEvent) -> Result<(), String> {
    if matches!(evt, FocusEvent::Pointer(FocusPointerEvent::Pointer(_, PointerAction::Move, _, _))) {
        Ok(())
    } else {
        Err(format!("Expected PointerAction::Move, got {:?}", evt))
    }
}

#[test]
fn mouse_click_in_region() {
    use flo_curves::arc::*;

    let test_program    = SubProgramId::called("focus_following_control");
    let scene           = Scene::default();

    // Couple of subprograms
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct SubProgram1(FocusEvent);
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct SubProgram2(FocusEvent);
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct CanvasProgram(FocusEvent);

    impl SceneMessage for SubProgram1 { }
    impl SceneMessage for SubProgram2 { }
    impl SceneMessage for CanvasProgram { }

    // Paths for our two subprograms
    let program1_path = Circle::new(UiPoint(300.0, 500.0), 100.0).to_path();
    let program2_path = Circle::new(UiPoint(700.0, 500.0), 100.0).to_path();

    let program1 = SubProgramId::called("Subprogram1");
    let program2 = SubProgramId::called("Subprogram2");
    let canvas   = SubProgramId::called("CanvasProgram");

    // Add test programs that relay the messages
    scene.add_subprogram(program1, |mut input, context| async move { while let Some(msg) = input.next().await { println!("Program1: {:?}", msg); context.send_message(SubProgram1(msg)).await.unwrap(); } }, 10);
    scene.add_subprogram(program2, |mut input, context| async move { while let Some(msg) = input.next().await { println!("Program2: {:?}", msg); context.send_message(SubProgram2(msg)).await.unwrap(); } }, 10);
    scene.add_subprogram(canvas, |mut input, context| async move { while let Some(msg) = input.next().await { println!("Canvas: {:?}", msg); context.send_message(CanvasProgram(msg)).await.unwrap(); } }, 10);

    // Create some points for mouse events
    let mut in_program1_path = PointerState::new();
    let mut in_program2_path = PointerState::new();
    let mut on_canvas        = PointerState::new();

    in_program1_path.location_in_canvas = Some((300.0, 500.0));
    in_program2_path.location_in_canvas = Some((700.0, 500.0));
    on_canvas.location_in_canvas        = Some((390.0, 590.0));

    let mut in_program1_path_button_down = in_program1_path.clone();
    let mut in_program2_path_button_down = in_program1_path.clone();
    let mut on_canvas_button_down        = in_program1_path.clone();

    in_program1_path_button_down.buttons = vec![Button::Left];
    in_program2_path_button_down.buttons = vec![Button::Left];
    on_canvas_button_down.buttons        = vec![Button::Left];

    let region1 = RegionId::new();
    let region2 = RegionId::new();

    TestBuilder::new()
        .send_message(Focus::ClaimRegion { program: program1, region_id: region1, region: vec![program1_path], z_index: 0 })
        .send_message(Focus::ClaimRegion { program: program2, region_id: region2, region: vec![program2_path], z_index: 1 })
        .send_message(Focus::SetCanvas(canvas))

        // Should keep tracking the mouse after the button goes down as staying in program 1
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::ButtonDown, PointerId(0), in_program1_path_button_down.clone())))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_enter(evt))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_buttondown(evt))
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::Move, PointerId(0), in_program2_path_button_down.clone())))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_move(evt))
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::ButtonUp, PointerId(0), in_program2_path.clone())))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_buttonup(evt))

        // Moves should get sent to whichever program they're over
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::Move, PointerId(0), in_program2_path.clone())))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_leave(evt))
        .expect_message(move |SubProgram2(evt): SubProgram2| expect_enter(evt))
        .expect_message(move |SubProgram2(evt): SubProgram2| expect_move(evt))
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::Move, PointerId(0), in_program2_path.clone())))
        .expect_message(move |SubProgram2(evt): SubProgram2| expect_move(evt))
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::Move, PointerId(0), in_program1_path.clone())))
        .expect_message(move |SubProgram2(evt): SubProgram2| expect_leave(evt))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_enter(evt))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_move(evt))
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::Move, PointerId(0), in_program1_path.clone())))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_move(evt))

        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::Move, PointerId(0), on_canvas.clone())))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_leave(evt))
        .expect_message(move |CanvasProgram(evt): CanvasProgram| expect_enter(evt))
        .expect_message(move |CanvasProgram(evt): CanvasProgram| expect_move(evt))
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::Move, PointerId(0), on_canvas.clone())))
        .expect_message(move |CanvasProgram(evt): CanvasProgram| expect_move(evt))

        .run_in_scene(&scene, test_program);        // No threads because otherwise the switch between programs is unreliable
}

#[test]
fn mouse_click_in_control_region() {
    use flo_curves::arc::*;

    let test_program    = SubProgramId::called("focus_following_control");
    let scene           = Scene::default();

    // Couple of subprograms
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct SubProgram1(FocusEvent);
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct SubProgram2(FocusEvent);
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct CanvasProgram(FocusEvent);

    impl SceneMessage for SubProgram1 { }
    impl SceneMessage for SubProgram2 { }
    impl SceneMessage for CanvasProgram { }

    // Paths for our two subprograms
    let program1_path = Circle::new(UiPoint(300.0, 500.0), 100.0).to_path();
    let program2_path = Circle::new(UiPoint(700.0, 500.0), 100.0).to_path();

    let program1 = SubProgramId::called("Subprogram1");
    let program2 = SubProgramId::called("Subprogram2");
    let canvas   = SubProgramId::called("CanvasProgram");

    let control1 = ControlId::new();
    let control2 = ControlId::new();

    // Add test programs that relay the messages
    scene.add_subprogram(program1, |mut input, context| async move { while let Some(msg) = input.next().await { println!("Program1: {:?}", msg); context.send_message(SubProgram1(msg)).await.unwrap(); } }, 10);
    scene.add_subprogram(program2, |mut input, context| async move { while let Some(msg) = input.next().await { println!("Program2: {:?}", msg); context.send_message(SubProgram2(msg)).await.unwrap(); } }, 10);
    scene.add_subprogram(canvas, |mut input, context| async move { while let Some(msg) = input.next().await { println!("Canvas: {:?}", msg); context.send_message(CanvasProgram(msg)).await.unwrap(); } }, 10);

    // Create some points for mouse events
    let mut in_program1_path = PointerState::new();
    let mut in_program2_path = PointerState::new();
    let mut on_canvas        = PointerState::new();

    in_program1_path.location_in_canvas = Some((300.0, 500.0));
    in_program2_path.location_in_canvas = Some((700.0, 500.0));
    on_canvas.location_in_canvas        = Some((390.0, 590.0));

    let mut in_program1_path_button_down = in_program1_path.clone();
    let mut in_program2_path_button_down = in_program1_path.clone();
    let mut on_canvas_button_down        = in_program1_path.clone();

    in_program1_path_button_down.buttons = vec![Button::Left];
    in_program2_path_button_down.buttons = vec![Button::Left];
    on_canvas_button_down.buttons        = vec![Button::Left];

    let region1 = RegionId::new();

    TestBuilder::new()
        .send_message(Focus::ClaimControlRegion { program: program1, region_id: region1, control: control1, region: vec![program1_path], z_index: 0 })
        .send_message(Focus::ClaimControlRegion { program: program1, region_id: region1, control: control2, region: vec![program2_path], z_index: 1 })
        .send_message(Focus::SetCanvas(canvas))

        // Should keep tracking the mouse after the button goes down as staying in program 1
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::ButtonDown, PointerId(0), in_program1_path_button_down.clone())))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_enter_program(evt))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_enter_control(evt, control1))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_buttondown(evt))
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::Move, PointerId(0), in_program2_path_button_down.clone())))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_move(evt))
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::ButtonUp, PointerId(0), in_program2_path.clone())))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_buttonup(evt))

        // Moves should get sent to whichever program they're over
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::Move, PointerId(0), in_program2_path.clone())))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_leave(evt))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_enter_control(evt, control2))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_move(evt))
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::Move, PointerId(0), in_program2_path.clone())))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_move(evt))
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::Move, PointerId(0), in_program1_path.clone())))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_leave(evt))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_enter_control(evt, control1))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_move(evt))
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::Move, PointerId(0), in_program1_path.clone())))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_move(evt))

        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::Move, PointerId(0), on_canvas.clone())))
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_leave(evt)) // Leave control
        .expect_message(move |SubProgram1(evt): SubProgram1| expect_leave(evt)) // Leave program
        .expect_message(move |CanvasProgram(evt): CanvasProgram| expect_enter(evt)) // Enter canvas
        .expect_message(move |CanvasProgram(evt): CanvasProgram| expect_move(evt))
        .send_message(Focus::Event(DrawEvent::Pointer(PointerAction::Move, PointerId(0), on_canvas.clone())))
        .expect_message(move |CanvasProgram(evt): CanvasProgram| expect_move(evt))

        .run_in_scene(&scene, test_program);        // No threads because otherwise the switch between programs is unreliable
}
