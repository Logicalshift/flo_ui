use flo_draw::canvas::*;
use flo_draw::canvas::scenery::*;
use flo_scene::*;

use futures::prelude::*;
use futures::future::{BoxFuture};

use std::sync::*;

///
/// Trait that adds extra functions to the `SceneContext` type to provide easier access to some types of message
///
pub trait UiSceneContextExt {
    ///
    /// Gathers drawing instructions to send to the active window
    ///
    /// Note that `Vec<Draw>` implements `GraphicsContext` so as well as pushing events like `Draw::FillColor` directly to
    /// it, you can call functions like `draw.fill_color()`.
    ///
    /// Returned future needs to be boxed due to Rust type limitations (can't declare a future type parameter with a
    /// lifetime like this)
    ///
    fn draw<'a>(&'a self, draw_fn: impl 'a + Send + for<'b> FnOnce(&'b mut Vec<Draw>) -> BoxFuture<'b, ()>) -> BoxFuture<'a, ()>;
}

impl UiSceneContextExt for SceneContext {
    fn draw<'a>(&'a self, draw_fn: impl 'a + Send + for<'b> FnOnce(&'b mut Vec<Draw>) -> BoxFuture<'b, ()>) -> BoxFuture<'a, ()> {
        async move {
            // Request the drawing instructions
            let mut drawing = vec![];
            draw_fn(&mut drawing).await;

            // Send to the context
            self.send_message(DrawingRequest::Draw(Arc::new(drawing))).await.ok();
        }.boxed()
    }
}
