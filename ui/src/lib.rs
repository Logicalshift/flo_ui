pub mod dialog;
pub mod focus;
pub mod subprograms;
pub mod util;

/// flo_draw provides basic window rendering and event handling
pub use flo_draw            as draw;

/// flo_canvas provides 2D rendering instructions and transformations (including things like text layout)
///
/// It provides layering and namespaces, which can be used to share a single 2D canvas amongst many
/// subprograms with minimal coupling.
pub use flo_draw::canvas    as canvas;

/// flo_scene is a runtime that divides software into 'subprograms' which communicate via messages
///
/// This is a way to build large programs out of small programs, and is used by this crate to 
/// route events and run things like dialogs 'in the background'. As well as providing a messaging
/// layer, it separates the connections between subprograms from the subprograms themselves, so
/// a subprogram can send, for instance, a drawing request without needing to know what precisely
/// will be handling it.
///
/// It's similar to the concepts of actors, or processes in a language like Erlang. It's not quite
/// either: it's core conceit is about isolating subprograms from the larger system, so they can be
/// focused on a single problem without needing to 'know' anything else about the environment they
/// find themselves in.
pub use flo_scene           as scene;

/// flo_binding is a reactive data binding library
pub use flo_binding         as binding;

/// flo_scene_binding provides flo_scene subprograms that react to changes to flo_binding bindings
pub use flo_scene_binding   as scene_binding;

/// flo_render_software can be used to render flo_canvas instructions offscreen
pub use flo_render_software as render2d;
