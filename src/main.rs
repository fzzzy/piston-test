
#[cfg(feature = "include_glfw")]
use glfw_window::GlfwWindow as AppWindow;
#[cfg(feature = "include_glutin")]
use glutin_window::GlutinWindow as AppWindow;
use graphics::{ Context, Graphics };
use opengl_graphics::{ GlGraphics, OpenGL };
use piston::window::{ Window, WindowSettings };
use piston::input::*;
use piston::event_loop::*;
use sdl2::audio::{AudioCallback, AudioFormat, AudioSpecDesired, AudioSpecWAV, AudioCVT};
#[cfg(feature = "include_sdl2")]
use sdl2_window::Sdl2Window as AppWindow;
use std::borrow::Cow;
use std::path::{PathBuf, Path};
use std::sync::mpsc::{ channel, Receiver };


struct Sound {
    data: Vec<u8>,
    volume: f32,
    begin: usize,
    begin_chan: Receiver<f64>,
    end: usize,
    end_chan: Receiver<f64>,
    pos: usize
}

impl AudioCallback for Sound {
    type Channel = u8;

    fn callback(&mut self, out: &mut [u8]) {
        let len = self.data.len();
        if self.begin == 0 && self.end == 0 {
            self.end = len;
        }

        let mut br = false;
        while !br {
            match self.begin_chan.try_recv() {
                Ok(val) => {
                    self.begin = (len as f64 * val) as usize;
                }
                Err(_e) => {
                    br = true;
                }
            }
        }
        br = false;
        while !br {
            match self.end_chan.try_recv() {
                Ok(val) => {
                    self.end = (len as f64 * val) as usize;
                }
                Err(_e) => {
                    br = true;
                }
            }
        }

        for dst in out.iter_mut() {
            if self.pos > self.end {
                self.pos = self.begin;
            }
            *dst = (*self.data.get(self.pos).unwrap_or(&0) as f32 * self.volume) as u8;
            self.pos += 1;
        }
    }
}

fn main() {
    let wav_file = Cow::from(Path::new("./assets/amen.wav"));

    let sdl_context = sdl2::init().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();
    let mut begin = 0 as f64;
    let mut end = 1 as f64;
    let mut button_down = false;
    let (txbegin, rxbegin) = channel();
    let (txend, rxend) = channel();

    let desired_spec = AudioSpecDesired {
        freq: Some(44_100),
        channels: Some(1),  // mono
        samples: None       // default sample size
    };
    let wav = AudioSpecWAV::load_wav(wav_file)
        .expect("Could not load test WAV file");
    let waveform_data = wav.buffer();
    let waveform_data_length = waveform_data.len();

    let displaycvt = AudioCVT::new(
        wav.format, wav.channels, wav.freq,
        AudioFormat::U8, 1, 8000
    ).expect("Could not convert WAV file for display");
    let display_data = displaycvt.convert(wav.buffer().to_vec());
    let display_length = display_data.len();

    let device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        let cvt = AudioCVT::new(
            wav.format, wav.channels, wav.freq,
            spec.format, spec.channels, spec.freq)
        .expect("Could not convert WAV file");
        let audio_data = cvt.convert(wav.buffer().to_vec());
        Sound {
            data: audio_data,
            volume: 0.25,
            begin: 0,
            begin_chan: rxbegin,
            end: 0,
            end_chan: rxend,
            pos: 0,
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
            button_down = true;
            println!("Pressed mouse button '{:?}'", button);
            let ratio = cursor[0] / 1024.0;
            println!("ratios {:?} {:?}", (ratio - begin).abs(), (ratio - end).abs());
            if (ratio - begin).abs() < (ratio - end).abs() {
                begin = ratio;
                txbegin.send(ratio);
            } else {
                end = ratio;
                txend.send(ratio);
            }
        }
        if let Some(Button::Keyboard(key)) = e.press_args() {
            println!("Pressed keyboard key '{:?}'", key);
        };
        if let Some(args) = e.button_args() {
            //println!("Scancode {:?}", args.scancode);
        }
        if let Some(button) = e.release_args() {
            match button {
                Button::Keyboard(key) => println!("Released keyboard key '{:?}'", key),
                Button::Mouse(button) => {
                    println!("Released mouse button '{:?}'", button);
                    button_down = false;
                },
                Button::Controller(button) => println!("Released controller button '{:?}'", button),
                Button::Hat(hat) => println!("Released controller hat `{:?}`", hat),
            }
        };
        e.mouse_cursor(|x, y| {
            cursor = [x, y];
            if button_down {
                let ratio = cursor[0] / 1024.0;
                println!("ratios {:?} {:?}", (ratio - begin).abs(), (ratio - end).abs());
                if (ratio - begin).abs() < (ratio - end).abs() {
                    begin = ratio;
                    txbegin.send(ratio);
                } else {
                    end = ratio;
                    txend.send(ratio);
                }
            }

            //println!("Mouse moved '{} {}'", x, y);
        });
        e.mouse_scroll(|dx, dy| println!("Scrolled mouse '{}, {}'", dx, dy));
        //e.mouse_relative(|dx, dy| println!("Relative mouse moved '{} {}'", dx, dy));
        e.text(|text| println!("Typed '{}'", text));
        e.resize(|w, h| println!("Resized '{}, {}'", w, h));
        if let Some(cursor) = e.cursor_args() {
            if cursor { println!("Mouse entered"); }
            else { println!("Mouse left"); }
        };
        if let Some(args) = e.render_args() {
            // println!("Render {}", args.ext_dt);
            gl.draw(args.viewport(), |c, g| {
                let full_width = window.size().width;
                let one_quarter = full_width as f64 / 4.0;
                let full_height = window.size().height;
                let half_height = full_height / 2.0;

                graphics::clear([1.0; 4], g);
                let wave = graphics::Line::new([0.5, 0.5, 0.5, 1.0], 1.0);

                for xval in 1..full_width as u32 {
                    let step = xval as f64 / full_width as f64;
                    let rawy = display_data[(display_length as f64 * step) as usize];
                    let yval = (rawy as f64 / 255.0 * full_height);
                    wave.draw([xval as f64, half_height, xval as f64, yval as f64],
                        &c.draw_state, c.transform, g);
                }
                draw_rectangles(begin, end, cursor, &window, &c, g);
            });
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
    begin: f64,
    end: f64,
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
    // let one_quarter = size.width as f64 / 4.0;
    line.draw([begin * size.width, 0.0, begin * size.width, size.height as f64],
        &c.draw_state, c.transform, g);
    line.draw([end * size.width, 0.0, end * size.width, size.height as f64],
        &c.draw_state, c.transform, g);
}
