use flo_ui::util::*;
use flo_ui::draw::*;
use flo_ui::canvas::*;
use flo_ui::scene::*;

use futures::prelude::*;

fn main() {
    with_2d_graphics(|| {
        // Create a window scene
        let window = create_window_scene(WindowProperties::new("Hello, world"));

        // Run a subprogram that renders 'Hello, world' to the window
        window.add_subprogram(SubProgramId::new(), |_input: InputStream<()>, context| async move {
            context.draw(|gc| async move {
                // Clear the canvas
                gc.clear_canvas(Color::Rgba(0.9, 1.0, 1.0, 1.0));

                // Define the coordinate scheme
                gc.canvas_height(1000.0);
                gc.center_region(0.0, 0.0, 1000.0, 1000.0);

                // Usually subprograms render to their own layers/namespaces so they don't interfere with one another. We'll use layer 0 because we're the only program.
                gc.layer(LayerId(0));
                gc.clear_layer();

                // Use the system font to render something
                gc.declare_font(FontId(1), FontSpec::system_ui_font());
                gc.set_font_size(FontId(1), 16.0);

                // Draw some centered text
                gc.fill_color(Color::Rgba(0.0, 0.0, 0.6, 1.0));
                gc.begin_line_layout(500.0, 500.0, TextAlignment::Center);
                gc.layout_text(FontId(1), "Hello, world".to_string());
                gc.draw_text_layout();
            }.boxed()).await;
        }, 1);
    });
}
