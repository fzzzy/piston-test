
#[cfg(feature = "include_glfw")]
use glfw_window::GlfwWindow as AppWindow;
#[cfg(feature = "include_glutin")]
use glutin_window::GlutinWindow as AppWindow;
use graphics::{ Context, Graphics };
use opengl_graphics::{ GlGraphics, OpenGL };
use piston::window::{ Window, WindowSettings };
use piston::input::*;
use piston::event_loop::*;
use sdl2::audio::{AudioCallback, AudioSpecDesired};
#[cfg(feature = "include_sdl2")]
use sdl2_window::Sdl2Window as AppWindow;
use std::sync::mpsc::{ channel, Receiver };

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
    sample_rate: f32,
    pitch_recv: Receiver<f32>,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        match self.pitch_recv.try_recv() {
            Ok(val) => {
                self.phase_inc = val / self.sample_rate;
            }
            Err(_err) => {}
        }
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 { self.volume } else { -self.volume };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();
    let (tx, rx) = channel();
    let mut pitch = 440.0;

    let desired_spec = AudioSpecDesired {
        freq: Some(44_100),
        channels: Some(1),  // mono
        samples: None       // default sample size
    };

    let device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        // Show obtained AudioSpec
        println!("{:?}", spec);

        // initialize the audio callback
        SquareWave {
            phase_inc: 440.0 / spec.freq as f32,
            phase: 0.0,
            volume: 0.125,
            sample_rate: spec.freq as f32,
            pitch_recv: rx,
        }
    }).unwrap();

    // Start playback
    device.resume();

    let opengl = OpenGL::V3_2;
    let mut window: AppWindow = WindowSettings::new("soundfarmer", [1024, 768])
        .exit_on_esc(true).opengl(opengl).build().unwrap();

    let ref mut gl = GlGraphics::new(opengl);
    let mut cursor = [0.0, 0.0];

    let mut events = Events::new(EventSettings::new().lazy(true));

    while let Some(e) = events.next(&mut window) {
        if let Some(Button::Mouse(button)) = e.press_args() {
            println!("Pressed mouse button '{:?}'", button);
        }
        if let Some(Button::Keyboard(key)) = e.press_args() {
            println!("Pressed keyboard key '{:?}'", key);
        };
        if let Some(args) = e.button_args() {
            println!("Scancode {:?}", args.scancode);
        }
        if let Some(button) = e.release_args() {
            match button {
                Button::Keyboard(key) => println!("Released keyboard key '{:?}'", key),
                Button::Mouse(button) => println!("Released mouse button '{:?}'", button),
                Button::Controller(button) => println!("Released controller button '{:?}'", button),
                Button::Hat(hat) => println!("Released controller hat `{:?}`", hat),
            }
        };
        e.mouse_cursor(|x, y| {
            cursor = [x, y];
            println!("Mouse moved '{} {}'", x, y);
            if (y.round() * 2.0) % 13.0 == 0.0 {
                let newpitch = (y.round() * 2.0) / 13.0 + 220.0;
                if pitch != newpitch {
                    pitch = newpitch;
                    tx.send(newpitch as f32);
                }
            }
        });
        e.mouse_scroll(|dx, dy| println!("Scrolled mouse '{}, {}'", dx, dy));
        e.mouse_relative(|dx, dy| println!("Relative mouse moved '{} {}'", dx, dy));
        e.text(|text| println!("Typed '{}'", text));
        e.resize(|w, h| println!("Resized '{}, {}'", w, h));
        if let Some(cursor) = e.cursor_args() {
            if cursor { println!("Mouse entered"); }
            else { println!("Mouse left"); }
        };
        if let Some(args) = e.render_args() {
            // println!("Render {}", args.ext_dt);
            gl.draw(args.viewport(), |c, g| {
                    graphics::clear([1.0; 4], g);
                    draw_rectangles(cursor, &window, &c, g);
                }
            );
        }
        if let Some(_args) = e.idle_args() {
            // println!("Idle {}", _args.dt);
        }
        if let Some(_args) = e.update_args() {
            /*
            // Used to test CPU overload.
            println!("Update {}", _args.dt);
            let mut x: f64 = 0.0;
            for _ in 0..500_000 {
                x += (1.0 + x).sqrt();
            }
            println!("{}", x);
            */
        }
    }
}

fn draw_rectangles<G: Graphics>(
    cursor: [f64; 2],
    window: &Window,
    c: &Context,
    g: &mut G,
) {
    let size = window.size();
    let zoom = 1.0;
    let offset = 0.0;

    let cursor_color = [0.0, 0.0, 0.0, 1.0];
    let zoomed_cursor = [offset + cursor[0] * zoom, offset + cursor[1] * zoom];
    graphics::ellipse(
        cursor_color,
        graphics::ellipse::circle(zoomed_cursor[0], zoomed_cursor[1], 4.0),
        c.transform,
        g
    );

    let rect_border = graphics::Rectangle::new_border([1.0, 0.0, 0.0, 1.0], 1.0);
    rect_border.draw([
            offset,
            offset,
            size.width as f64 * zoom - 1.0,
            size.height as f64 * zoom - 1.0
        ],
        &c.draw_state, c.transform, g);

    let line = graphics::Line::new([0.5, 0.5, 0.5, 1.0], 1.0);
    line.draw([0.0, 0.0, size.width as f64, size.height as f64],
        &c.draw_state, c.transform, g);
    line.draw([size.width as f64, 0.0, 0.0, size.height as f64],
        &c.draw_state, c.transform, g);
}
