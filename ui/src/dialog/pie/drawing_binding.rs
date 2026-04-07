use super::point_mapping::*;

use flo_binding::*;
use flo_draw::canvas::*;
use flo_draw::canvas::scenery::*;
use flo_scene::*;

use futures::prelude::*;

use std::sync::*;

///
/// Receives 'flat' drawing instructions from the drawing_stream, and processes them, changing 'draw_binding' to be up to date with the 'mapped' drawing instructions
///
/// Positive 'y' coordinates are a distance from the inner radius of the dialog. 'x' coordinates are a distance around the circumference (where 'x=0' is the center
/// of the dialog)
///
pub async fn pie_dialog_drawing_binding_program(input: InputStream<()>, context: SceneContext, draw_binding: Binding<Arc<Vec<Draw>>>, point_mapping: PieDialogPointMapping, drawing_stream: impl 'static + Unpin + Send + Stream<Item=Draw>) {
    // We just monitor the drawing stream
    drop(input);

    let mut drawing             = drawing_stream.ready_chunks(10_000);
    let Ok(mut draw_immediate)  = context.send(()) else { return; };

    while let Some(new_instructions) = drawing.next().await {
        // Split up/process the drawing instructions
        let mut cleared             = false;
        let mut shapes              = vec![];
        let mut other_instructions  = vec![];

        for new_draw in new_instructions.into_iter() {
            match &new_draw {
                // Path operations
                Draw::Path(_)                       |
                Draw::Fill                          |
                Draw::Stroke                        |
                Draw::LineWidth(_)                  |
                Draw::LineWidthPixels(_)            |
                Draw::LineJoin(_)                   |
                Draw::LineCap(_)                    |
                Draw::NewDashPattern                |
                Draw::DashLength(_)                 |
                Draw::DashOffset(_)                 |
                Draw::FillColor(_)                  |
                Draw::FillTexture(_, _, _)          |
                Draw::FillGradient(_, _, _)         |
                Draw::FillTransform(_)              |
                Draw::StrokeColor(_)                |
                Draw::WindingRule(_)                |
                Draw::BlendMode(_)                  => { shapes.push(new_draw); }

                // Semi-supported for path state
                Draw::PushState                     |
                Draw::PopState                      => { shapes.push(new_draw.clone()); other_instructions.push(new_draw); }

                // Clearing (we just assume everything clears the layer)
                Draw::ClearCanvas(_)                |
                Draw::ClearLayer                    |
                Draw::ClearAllLayers                => { cleared = true; shapes.clear(); }

                // Resource ops, evaluated immediately
                Draw::Sprite(_)                     |
                Draw::MoveSpriteFrom(_)             |
                Draw::ClearSprite                   |
                Draw::SpriteTransform(_)            |
                Draw::DrawSprite(_)                 |
                Draw::DrawSpriteWithFilters(_, _)   |
                Draw::Texture(_, _)                 |
                Draw::Font(_, _)                    |
                Draw::Gradient(_, _)                => { other_instructions.push(new_draw); }

                // Unsupported operations
                Draw::StartFrame                    |
                Draw::ShowFrame                     |
                Draw::ResetFrame                    |
                Draw::IdentityTransform             |
                Draw::CanvasHeight(_)               |
                Draw::CenterRegion(_, _)            |
                Draw::MultiplyTransform(_)          |
                Draw::Unclip                        |
                Draw::Clip                          |
                Draw::Store                         |
                Draw::Restore                       |
                Draw::FreeStoredBuffer              |
                Draw::Layer(_)                      |
                Draw::LayerBlend(_, _)              |
                Draw::LayerAlpha(_, _)              |
                Draw::SetLayerTransform(_)          |
                Draw::SwapLayers(_, _)              |
                Draw::PlaceLayerBefore(_, _)        |
                Draw::BeginLineLayout(_, _, _)      |
                Draw::DrawLaidOutText               |
                Draw::DrawText(_, _, _, _)          |
                Draw::Namespace(_)                  => { }
            }
        }

        // Send the resource instructions immediately
        if !other_instructions.is_empty() {
            draw_immediate.send(DrawingRequest::Draw(Arc::new(other_instructions))).await.ok();
        }

        // Copy the old drawing (unless there was a 'clear' instruction, in which case just discard it)
        let mut new_drawing = if cleared { vec![] } else { draw_binding.get().iter().cloned().collect() };

        // Transform the shapes
        if !shapes.is_empty() {
            let transformed_shapes = point_mapping.transform_paths(stream::iter(shapes)).await;
            new_drawing.extend(transformed_shapes);
        }

        // Store as the new drawing binding
        draw_binding.set(Arc::new(new_drawing));
    }
}
