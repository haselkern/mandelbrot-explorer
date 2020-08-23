mod render;

use ggez::conf::{NumSamples, WindowMode, WindowSetup};
use ggez::event::MouseButton;
use ggez::graphics;
use ggez::graphics::{Canvas, Color, DrawParam, Image};
use ggez::input::mouse;
use ggez::mint::Point2;
use ggez::{event, Context};
use num_complex::Complex32;
use std::ops::Mul;
use std::sync::mpsc::Receiver;

// TODO use the actual window size and let other aspect ratios happen
const WINDOW_SIZE: i32 = 512;

/// Contains the (upper left, lower right) bounds
#[derive(Clone)]
pub struct ViewBounds(Complex32, Complex32);

struct MainState {
    receiver: Receiver<render::Point>,
    received_pixels: i32,
    canvas: Canvas,
    bounds: Vec<ViewBounds>,
    /// (Upper left corner, size)
    zoom_box: Option<(Point2<f32>, f32)>,
}

impl MainState {
    fn new(ctx: &mut Context) -> ggez::GameResult<MainState> {
        let initial_bounds = ViewBounds(Complex32::new(-2.0, -1.5), Complex32::new(1.0, 1.5));

        Ok(MainState {
            receiver: render::start_thread(initial_bounds.clone()),
            received_pixels: 0,
            canvas: Canvas::new(ctx, WINDOW_SIZE as u16, WINDOW_SIZE as u16, NumSamples::One)?,
            bounds: vec![initial_bounds],
            zoom_box: None,
        })
    }

    fn rerender(&mut self, ctx: &mut Context) -> ggez::GameResult {
        self.receiver = render::start_thread(self.bounds.last().unwrap().clone());
        self.canvas = Canvas::new(ctx, WINDOW_SIZE as u16, WINDOW_SIZE as u16, NumSamples::One)?;
        self.received_pixels = 0;
        Ok(())
    }
}

impl event::EventHandler for MainState {
    fn update(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        if let Some(zoom) = &mut self.zoom_box {
            let mpos = mouse::position(ctx);
            let (dx, dy) = (mpos.x - zoom.0.x, mpos.y - zoom.0.y);
            zoom.1 = dx.abs().max(dy.abs()).max(10.0);
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        graphics::clear(ctx, Color::from_rgb(0, 0, 0));

        // Receive points from the rendering thread and draw them on an offscreen canvas
        let mut max = 64; // Only receive this many pixels at most, to prevent blocking
        let pixel = Image::solid(ctx, 1, Color::from_rgb(255, 255, 255))?;
        // TODO is this loop the bottleneck?
        graphics::set_canvas(ctx, Some(&self.canvas));
        while let Ok(p) = self.receiver.try_recv() {
            self.received_pixels += 1;
            max -= 1;
            if max <= 0 {
                break;
            }
            if let Some(i) = p.i {
                let v = (i * 255 / 100) as u8;
                graphics::draw(
                    ctx,
                    &pixel,
                    DrawParam::default()
                        .dest(p.p)
                        .color(Color::from_rgb(v, v, v)),
                )?;
            }
        }
        graphics::set_canvas(ctx, None);

        // Draw the drawn mandelbrot set
        graphics::draw(ctx, &self.canvas, DrawParam::default())?;

        // Draw a progress bar
        if self.received_pixels > 0 && self.received_pixels < WINDOW_SIZE * WINDOW_SIZE {
            let progress = (self.received_pixels / WINDOW_SIZE) as u16;
            if progress > 0 {
                let progress = Image::solid(ctx, progress, Color::from_rgb(0, 200, 0))?;
                graphics::draw(
                    ctx,
                    &progress,
                    DrawParam::default().dest([0.0, WINDOW_SIZE as f32 - 5.0]),
                )?;
            }
        }

        // Draw zoom box overlay
        if let Some(zoom) = self.zoom_box {
            let b = Image::solid(ctx, zoom.1 as u16, Color::from_rgba(255, 255, 255, 128))?;
            graphics::draw(ctx, &b, DrawParam::default().dest(zoom.0))?;
        }

        graphics::present(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        // Zoom in with left click
        if button == MouseButton::Left {
            self.zoom_box = Some(([x, y].into(), 1.0));
        }
    }

    fn mouse_button_up_event(&mut self, ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) {
        if button == MouseButton::Left {
            // Zoom in with left click
            if let Some(zoom) = self.zoom_box {
                let bounds = self.bounds.last().unwrap();
                let x = (zoom.0.x / WINDOW_SIZE as f32) * (bounds.1.re - bounds.0.re) + bounds.0.re;
                let y = (zoom.0.y / WINDOW_SIZE as f32) * (bounds.1.im - bounds.0.im) + bounds.0.im;
                let d = (zoom.1 / WINDOW_SIZE as f32) * (bounds.1.re - bounds.0.re);
                self.bounds.push(ViewBounds(
                    Complex32::new(x, y),
                    Complex32::new(x + d, y + d),
                ));
                self.rerender(ctx).unwrap();
                self.zoom_box = None;
            }
        } else if button == MouseButton::Right {
            // Zoom out with right click
            if self.bounds.len() > 1 {
                self.bounds.pop();
                self.rerender(ctx).unwrap();
            } else {
                let bounds = &self.bounds[0];
                self.bounds[0] = ViewBounds(bounds.0.mul(2.0), bounds.1.mul(2.0));
                self.rerender(ctx).unwrap();
            }
        }
    }
}

fn main() -> ggez::GameResult {
    let cb = ggez::ContextBuilder::new("haselkern.mandelbrot", "haselkern")
        .window_setup(WindowSetup::default().title("Mandelbrot Explorer"))
        .window_mode(WindowMode::default().dimensions(512.0, 512.0));
    let (ctx, event_loop) = &mut cb.build()?;
    let state = &mut MainState::new(ctx)?;
    event::run(ctx, event_loop, state)
}
