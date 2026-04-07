use super::pie_animation::*;
use crate::subprograms::*;
use crate::util::*;

use flo_binding::*;
use flo_scene::*;
use flo_scene::programs::*;
use flo_draw::canvas::*;
use flo_draw::canvas::scenery::*;
use flo_curves::*;
use flo_curves::bezier::*;

use futures::prelude::*;
use futures::channel::mpsc;

use std::f64;
use std::sync::*;

///
/// Configuration and a way to run pie dialogs
///
#[derive(Clone, Debug)]
pub struct PieDialogProgram {
    /// The animation used to show and hide this dialog
    animation:      PieAnimation,

    /// Where the center of the pie is located
    center:         UiPoint,

    /// The inner radius of the pie slice
    inner_radius:   f64,

    /// The outer radius of the pie slice
    outer_radius:   f64,

    /// The angle of the slice, in radians
    angle:          f64,

    /// The 'width' of the slice in pixels (number of pixels from the flat coordinate scheme that covers the whole range)
    width:          f64,

    /// The title displayed on the outside of the slice
    title:          String,

    /// If true, there's a '...' icon for configuring more details for this program
    has_detail:     bool,

    /// The namespace where this pie is rendered
    namespace:      NamespaceId,

    /// The layer where this pie slice is rendered
    layer:          LayerId,
}

///
/// Describes how a point is mapped in a pie dialog
///
#[derive(Clone, Copy, Debug)]
struct PieDialogPointMapping {
    inner_radius:   f64,
    angle:          f64,
    center:         UiPoint,
}

impl Default for PieDialogProgram {
    fn default() -> Self {
        PieDialogProgram {
            namespace:      NamespaceId::new(),
            center:         UiPoint(0.0, 0.0),
            layer:          LayerId(0),
            animation:      PieAnimation::ExpandFanOut,
            inner_radius:   32.0,
            outer_radius:   100.0,
            angle:          f64::consts::PI/2.0,
            width:          100.0,
            title:          String::default(),
            has_detail:     false,
        }
    }
}

// Config

impl PieDialogProgram {
    ///
    /// Creates a new pie dialog with a layer and a namespace
    ///
    pub fn new(center: impl Coordinate + Coordinate2D, angle_degrees: f64, namespace: NamespaceId, layer: LayerId) -> Self {
        Self::default()
            .with_center(center)
            .with_angle(angle_degrees)
            .with_namespace(namespace)
            .with_layer(layer)
    }

    ///
    /// Sets the center position of the pie
    ///
    #[inline]
    pub fn with_center(mut self, center: impl Coordinate + Coordinate2D) -> Self {
        self.center = UiPoint::from_components(&[center.x(), center.y()]);
        self
    }

    ///
    /// Sets the namespace to use for rendering the pie slice
    ///
    #[inline]
    pub fn with_namespace(mut self, new_namespace: NamespaceId) -> Self {
        self.namespace = new_namespace;
        self
    }

    ///
    /// Sets the layer to use for rendering the pie slice
    ///
    #[inline]
    pub fn with_layer(mut self, new_layer: LayerId) -> Self {
        self.layer = new_layer;
        self
    }

    ///
    /// Sets the animation for the program
    ///
    #[inline]
    pub fn with_animation(mut self, new_animation: PieAnimation) -> Self {
        self.animation = new_animation;
        self
    }

    ///
    /// Sets the inner radius of the pie slice
    ///
    #[inline]
    pub fn with_inner_radius(mut self, new_radius: f64) -> Self {
        self.inner_radius = new_radius;
        self
    }

    ///
    /// Sets the outer radius of the pie slice
    ///
    #[inline]
    pub fn with_outer_radius(mut self, new_radius: f64) -> Self {
        self.outer_radius = new_radius;
        self
    }

    ///
    /// Sets the angle of the slice (specified in degrees, converted to radians)
    ///
    #[inline]
    pub fn with_angle(mut self, new_angle_degrees: f64) -> Self {
        self.angle = new_angle_degrees.to_radians();
        self
    }

    ///
    /// Sets the 'width' of the slice in pixels (number of pixels from the flat coordinate scheme that covers the whole range)
    ///
    #[inline]
    pub fn with_width(mut self, new_width: f64) -> Self {
        self.width = new_width;
        self
    }

    ///
    /// Sets the title displayed on the outside of the slice
    ///
    #[inline]
    pub fn with_title(mut self, new_title: String) -> Self {
        self.title = new_title;
        self
    }

    ///
    /// Sets whether there's a '...' icon for configuring more details for this program
    ///
    #[inline]
    pub fn with_has_detail(mut self, new_has_detail: bool) -> Self {
        self.has_detail = new_has_detail;
        self
    }

    ///
    /// Retrieves the mapping type for this program (which can be used to find how points map onto the pie layer)
    ///
    #[inline]
    pub fn point_mapping(&self) -> PieDialogPointMapping {
        PieDialogPointMapping { 
            inner_radius:   self.inner_radius, 
            angle:          self.angle,
            center:         self.center
        }
    }
}

impl PieDialogPointMapping {
    ///
    /// Maps a point from 'flat' space to 'pie' space
    ///
    #[inline]
    pub fn map_point<TCoord>(&self, pos: &TCoord) -> TCoord
    where 
        TCoord: Coordinate + Coordinate2D,
    {
        // The distance from the center is the y position plus the inner radius (so y=0 is the inner circle)
        let r = pos.y() + self.inner_radius;

        // The angle is 'x' distance around the pie from the 'angle'
        let theta = (pos.x()/(2.0*f64::consts::PI*r)) * 2.0*f64::consts::PI;
        let theta = self.angle + theta;

        // Calculate the new position from the old one
        let new_x = r * theta.sin() + self.center.x();
        let new_y = r * theta.cos() + self.center.y();

        TCoord::from_components(&[new_x, new_y])
    }

    ///
    /// Maps a point from 'pie' space to 'flat' space
    ///
    #[inline]
    pub fn unmap_point<TCoord>(&self, pos: &TCoord) -> TCoord
    where 
        TCoord: Coordinate + Coordinate2D,
    {
        let dx = pos.x() - self.center.x();
        let dy = pos.y() - self.center.y();

        let r     = (dx * dx + dy * dy).sqrt();
        let theta = dx.atan2(dy);

        let x = (theta - self.angle) * r;
        let y = r - self.inner_radius;

        TCoord::from_components(&[x, y])
    }

    ///
    /// Transforms any paths found in the supplied drawing, returning a new drawing (which just draws the paths)
    ///
    /// Things like layer clearing and resource operations will need to be processed separately from this.
    ///
    pub async fn transform_paths(&self, drawing: impl 'static + Send + Unpin + Stream<Item=Draw>) -> Vec<Draw> {
        let as_paths    = drawing_to_attributed_paths::<UiPath, _>(drawing);
        let transformed = as_paths.map(|(attributes, path_set)| {
                let new_paths = path_set.iter().flat_map(|path| distort_path::<_, _, UiPath>(path, |point, _, _| self.map_point(&point), 1.0, 0.1))
                    .collect::<Vec<_>>();
                (attributes, new_paths)
            });
        let redrawn     = transformed.flat_map(|(attributes, path_set)| {
            let mut draw = vec![];
            draw.render_bezier_shape(attributes.iter(), path_set.iter());
            stream::iter(draw)
        });

        redrawn.collect::<_>().await
    }
}

// Execution

impl PieDialogProgram {
    ///
    /// Runs the subprogram for this pie dialog
    ///
    pub async fn run(self, input: InputStream<PieDialog>, context: SceneContext) {
        let Some(our_program_id) = context.current_program_id() else { return };

        // Namespace and layer that we'll be drawing on
        let namespace   = self.namespace;
        let layer       = self.layer;

        // Original angle and center (used when the position changes)
        let original_center = self.center;
        let original_angle  = self.angle;

        // Binding for the instructions used to redraw the pie slice (coordinates transformed)
        let draw_binding = bind(Arc::<Vec<Draw>>::new(vec![]));

        // Current position is used to calculate the layer transform
        let current_pos = bind((original_center, original_angle));

        // Title is rendered at the outer radius
        let outer_radius = bind(self.outer_radius);
        let title        = bind(self.title.clone());

        // Stream for processing the draw instructions
        let (send_drawing, recv_drawing) = mpsc::channel::<Draw>(1000);

        let with_text_layout    = drawing_with_laid_out_text(recv_drawing);
        let with_glyph_paths    = drawing_with_text_as_paths(with_text_layout);

        // Tell SceneControl to create a subprogram to update the drawing binding whenever the redrawn paths is changed
        let update_draw_binding = draw_binding.clone();
        let point_mapping       = self.point_mapping();

        context.send_message(SceneControl::start_child_program(SubProgramId::new(), our_program_id, move |input: InputStream<()>, context| {
            async move {
                // We just monitor the drawing stream
                drop(input);

                let mut drawing             = with_glyph_paths.ready_chunks(10_000);
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
                    let mut new_drawing = if cleared { vec![] } else { update_draw_binding.get().iter().cloned().collect() };

                    // Transform the shapes
                    if !shapes.is_empty() {
                        let transformed_shapes = point_mapping.transform_paths(stream::iter(shapes)).await;
                        new_drawing.extend(transformed_shapes);
                    }

                    // Store as the new drawing binding
                    update_draw_binding.set(Arc::new(new_drawing));
                }
            }
        }, 1)).await.ok();

        // Process the input
        let mut input           = input;
        let mut send_drawing    = send_drawing;

        while let Some(msg) = input.next().await {
            match msg {
                PieDialog::SetPosition(new_center, new_angle) => {
                    current_pos.set((new_center, new_angle));
                },

                PieDialog::SetRadius(new_outer_radius) => {
                    outer_radius.set(new_outer_radius);
                },

                PieDialog::SetTitle(new_title) => {
                    title.set(new_title);
                },

                PieDialog::Draw(drawing) => {
                    send_drawing.send_all(&mut stream::iter(drawing.iter().map(|draw| Ok(draw.clone())))).await.ok();
                },

                PieDialog::ClaimRegion { program, region, control, z_index } => {
                    // Need to transform/recalculate the path and then add to the claims
                    // (Also need a subprogram to manage them)
                    todo!()
                },

                PieDialog::Close => {
                    // TODO: animation
                    break;
                },
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn map_zero_point() {
        let dialog_program = PieDialogProgram::default()
            .with_center(UiPoint(100.0, 100.0))
            .with_inner_radius(20.0)
            .with_outer_radius(100.0)
            .with_angle(0.0);

        let zero_point = dialog_program.point_mapping().map_point(&UiPoint(0.0, 0.0));
        assert!((zero_point.x()-100.0).abs() < 0.1, "{:?}", zero_point);
        assert!((zero_point.y()-120.0).abs() < 0.1, "{:?}", zero_point);
    }

    #[test]
    pub fn map_furthest_point() {
        let dialog_program = PieDialogProgram::default()
            .with_center(UiPoint(100.0, 100.0))
            .with_inner_radius(20.0)
            .with_outer_radius(100.0)
            .with_angle(0.0);

        let furthest_point = dialog_program.point_mapping().map_point(&UiPoint(0.0, 80.0));
        assert!((furthest_point.x()-100.0).abs() < 0.1, "{:?}", furthest_point);
        assert!((furthest_point.y()-200.0).abs() < 0.1, "{:?}", furthest_point);
    }

    #[test]
    pub fn unmap_points() {
        let dialog_program = PieDialogProgram::default()
            .with_center(UiPoint(100.0, 100.0))
            .with_inner_radius(20.0)
            .with_outer_radius(100.0)
            .with_angle(0.0);

        fn check_unmap(program: &PieDialogProgram, point: UiPoint) {
            let mapped_point    = program.point_mapping().map_point(&point);
            let unmapped_point  = program.point_mapping().unmap_point(&mapped_point);

            assert!((point.x() - unmapped_point.x() < 0.01), "{:?} != {:?}", point, unmapped_point);
            assert!((point.y() - unmapped_point.y() < 0.01), "{:?} != {:?}", point, unmapped_point);
        }

        check_unmap(&dialog_program, UiPoint(0.0, 0.0));
        check_unmap(&dialog_program, UiPoint(0.0, 100.0));
        check_unmap(&dialog_program, UiPoint(50.0, 100.0));
        check_unmap(&dialog_program, UiPoint(-50.0, 0.0));
    }
}
