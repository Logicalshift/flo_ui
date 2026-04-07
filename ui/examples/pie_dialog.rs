use flo_scene::*;
use flo_ui::util::*;
use flo_ui::draw::*;
use flo_ui::canvas::*;
use flo_ui::dialog::pie::*;
use flo_ui::subprograms::*;

use futures::prelude::*;

use std::sync::*;

fn main() {
    with_2d_graphics(|| {
        // Create a window scene
        let window = create_window_scene(WindowProperties::new("Hello, world"));

        // Run a pie dialog subprogram
        let pie_program = SubProgramId::new();
        let pie         = PieDialogProgram::new(UiPoint(500.0, 500.0), 45.0, NamespaceId::new(), LayerId(0))
            .with_inner_radius(20.0)
            .with_outer_radius(200.0)
            .with_title("Pie dialog");

        window.add_subprogram(pie_program, move |input, context| { pie.run(input, context) }, 1);

        // Run a subprogram that renders 'Hello, world' to the window
        window.add_subprogram(SubProgramId::new(), |_input: InputStream<()>, context| async move {
            // Set up the canvas
            context.draw(|gc| async move {
                // Clear the canvas
                gc.clear_canvas(Color::Rgba(0.9, 1.0, 1.0, 1.0));

                // Define the coordinate scheme
                gc.canvas_height(1000.0);
                gc.center_region(0.0, 0.0, 1000.0, 1000.0);

                // Usually subprograms render to their own layers/namespaces so they don't interfere with one another. We'll use layer 0 because we're the only program.
                gc.layer(LayerId(0));
                gc.clear_layer();

                // Draw a center circle for the pie dialog program
                gc.new_path();
                gc.circle(500.0, 500.0, 18.0);
                gc.fill_color(Color::Rgba(0.5, 0.8, 1.0, 1.0));
                gc.fill();
            }.boxed()).await;

            // Draw to the pie
            let mut pie         = context.send(pie_program).unwrap();
            let mut pie_drawing = vec![];

            // Draw a rectangle
            pie_drawing.new_path();
            pie_drawing.rect(-20.0, 0.0, 20.0, 180.0);
            pie_drawing.stroke_color(Color::Rgba(0.6, 0.6, 0.6, 1.0));
            pie_drawing.line_width_pixels(1.0);
            pie_drawing.stroke();

            // Write out 'hello world' in the pie diagram
            pie_drawing.declare_font(FontId(1), FontSpec::system_ui_font());
            pie_drawing.set_font_size(FontId(1), 18.0);

            // Draw some centered text
            pie_drawing.fill_color(Color::Rgba(0.0, 0.0, 0.6, 1.0));
            pie_drawing.begin_line_layout(0.0, 150.0, TextAlignment::Center);
            pie_drawing.layout_text(FontId(1), "Pie dialog".to_string());
            pie_drawing.draw_text_layout();

            pie_drawing.fill_color(Color::Rgba(0.0, 0.0, 0.6, 1.0));
            pie_drawing.begin_line_layout(0.0, 50.0, TextAlignment::Center);
            pie_drawing.layout_text(FontId(1), "Demonstration".to_string());
            pie_drawing.draw_text_layout();

            pie.send(PieDialog::Draw(Arc::new(pie_drawing))).await.ok();
        }, 1);
    });
}
