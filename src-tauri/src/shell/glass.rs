//! The Glass look's blur.
//!
//! Windows' own compositor draws a blurred copy of whatever is behind the panel, clipped to a
//! rounded rectangle; the page above it stays transparent and paints only the tint, the lit rims
//! and the shadows. Traywise never sees those pixels: the blur happens inside Windows, so the
//! "never read the desktop" rule still holds.
//!
//! The recipe was proven in a throwaway test window: a host backdrop brush on a sprite visual,
//! attached to the window through the desktop interop, with a dispatcher queue on the thread that
//! owns the window. It works with `no_redirection_bitmap`, focused or not. Windows decides how
//! strong the blur is; the page's tint sets the look.
//!
//! This module only creates and moves the blur; `panel.rs` decides when it exists.

use windows::{
    System::DispatcherQueueController,
    UI::Composition::{
        CompositionRoundedRectangleGeometry, Compositor, ContainerVisual, SpriteVisual,
        Desktop::DesktopWindowTarget,
    },
    Win32::{
        Foundation::HWND,
        Graphics::Dwm::{DWMWINDOWATTRIBUTE, DwmSetWindowAttribute},
        System::WinRT::{
            Composition::ICompositorDesktopInterop, CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT,
            DispatcherQueueOptions,
        },
    },
    core::{BOOL, Interface, Result},
};
use windows_numerics::{Vector2, Vector3};

/// DWMWA_USE_HOSTBACKDROPBRUSH: lets a visual use the blurred desktop behind this window.
const USE_HOST_BACKDROP_BRUSH: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(17);

/// A blurred, rounded rectangle behind one window. Keep it alive for as long as the blur should
/// show; dropping it removes the blur.
pub struct Glass {
    // Held only so they live as long as the sprite.
    _queue: Option<DispatcherQueueController>,
    compositor: Compositor,
    _target: DesktopWindowTarget,
    root: ContainerVisual,
    sprite: SpriteVisual,
    shape: CompositionRoundedRectangleGeometry,
    /// The clear variant's blurred pieces, one under each tile (made as needed, then reused).
    pieces: Vec<Piece>,
}

struct Piece {
    sprite: SpriteVisual,
    shape: CompositionRoundedRectangleGeometry,
}

/// One blurred piece of the clear variant: a tile, the header or the tab bar, in physical pixels in the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    pub opacity: f32,
}

// shortcut: the composition objects are free-threaded, but Traywise only touches them from the
// thread that owns the window; revisit if the blur ever moves between threads.
unsafe impl Send for Glass {}
unsafe impl Sync for Glass {}

/// Where the blur sits, in physical pixels inside the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
}

/// The window's area pulled in by `margin` on every side (the room a shadow needs), rounded by
/// `radius`. Never negative.
pub fn inset(window_width: f32, window_height: f32, margin: f32, radius: f32) -> Rect {
    Rect {
        x: margin,
        y: margin,
        width: (window_width - 2.0 * margin).max(0.0),
        height: (window_height - 2.0 * margin).max(0.0),
        radius,
    }
}

impl Glass {
    /// Puts a blurred rounded rectangle behind `hwnd`. Call on the thread that owns the window.
    pub fn attach(hwnd: HWND, rect: Rect) -> Result<Self> {
        unsafe {
            // The compositor needs a dispatcher queue on this thread. If one exists already this
            // fails, which is fine: the existing one is used.
            let options = DispatcherQueueOptions {
                dwSize: size_of::<DispatcherQueueOptions>() as u32,
                threadType: DQTYPE_THREAD_CURRENT,
                apartmentType: DQTAT_COM_NONE,
            };
            let queue = CreateDispatcherQueueController(options).ok();

            let on = BOOL(1);
            DwmSetWindowAttribute(hwnd, USE_HOST_BACKDROP_BRUSH, &on as *const _ as *const _, size_of::<BOOL>() as u32)?;

            let compositor = Compositor::new()?;
            let interop: ICompositorDesktopInterop = compositor.cast()?;
            let target = interop.CreateDesktopWindowTarget(hwnd, false)?;
            let root = compositor.CreateContainerVisual()?;
            let sprite = compositor.CreateSpriteVisual()?;
            sprite.SetBrush(&compositor.CreateHostBackdropBrush()?)?;

            let shape = compositor.CreateRoundedRectangleGeometry()?;
            let clip = compositor.CreateGeometricClip()?;
            clip.SetGeometry(&shape)?;
            sprite.SetClip(&clip)?;

            root.Children()?.InsertAtTop(&sprite)?;
            target.SetRoot(&root)?;

            let glass = Self { _queue: queue, compositor, _target: target, root, sprite, shape, pieces: Vec::new() };
            glass.set_rect(rect)?;
            Ok(glass)
        }
    }

    /// Moves and resizes the blur (after the window or the panel changes size).
    pub fn set_rect(&self, rect: Rect) -> Result<()> {
        self.sprite.SetOffset(Vector3 { X: rect.x, Y: rect.y, Z: 0.0 })?;
        let size = Vector2 { X: rect.width, Y: rect.height };
        self.sprite.SetSize(size)?;
        self.shape.SetSize(size)?;
        self.shape.SetCornerRadius(Vector2 { X: rect.radius, Y: rect.radius })
    }

    /// The clear variant: blur only under these shapes (and none anywhere else). Pieces are made as
    /// needed and reused; the ones not needed are hidden. An empty list shows no blur at all.
    pub fn set_shapes(&mut self, shapes: &[Shape]) -> Result<()> {
        while self.pieces.len() < shapes.len() {
            let piece = self.new_piece()?;
            self.pieces.push(piece);
        }
        for (i, piece) in self.pieces.iter().enumerate() {
            let Some(s) = shapes.get(i) else {
                piece.sprite.SetIsVisible(false)?;
                continue;
            };
            let size = Vector2 { X: s.width, Y: s.height };
            piece.sprite.SetOffset(Vector3 { X: s.x, Y: s.y, Z: 0.0 })?;
            piece.sprite.SetSize(size)?;
            piece.sprite.SetOpacity(s.opacity)?;
            piece.shape.SetSize(size)?;
            let r = s.radius.min(s.width / 2.0).min(s.height / 2.0);
            piece.shape.SetCornerRadius(Vector2 { X: r, Y: r })?;
            piece.sprite.SetIsVisible(true)?;
        }
        Ok(())
    }

    fn new_piece(&self) -> Result<Piece> {
        let sprite = self.compositor.CreateSpriteVisual()?;
        sprite.SetBrush(&self.compositor.CreateHostBackdropBrush()?)?;
        let shape = self.compositor.CreateRoundedRectangleGeometry()?;
        let clip = self.compositor.CreateGeometricClip()?;
        clip.SetGeometry(&shape)?;
        sprite.SetClip(&clip)?;
        sprite.SetIsVisible(false)?;
        self.root.Children()?.InsertAtTop(&sprite)?;
        Ok(Piece { sprite, shape })
    }

    /// Shows or hides the blur without tearing it down. A window can only be given its composition
    /// target once, so the blur is made once and hidden when the look doesn't use it.
    pub fn set_visible(&self, visible: bool) -> Result<()> {
        self.sprite.SetIsVisible(visible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inset_pulls_the_blur_in_from_every_side() {
        let r = inset(450.0, 810.0, 15.0, 20.0);
        assert_eq!((r.x, r.y, r.width, r.height, r.radius), (15.0, 15.0, 420.0, 780.0, 20.0));
    }

    #[test]
    fn inset_never_goes_negative() {
        let r = inset(10.0, 10.0, 30.0, 5.0);
        assert_eq!((r.width, r.height), (0.0, 0.0));
    }
}
