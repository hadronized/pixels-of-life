use glfw::{Action, Context as _, Key, MouseButton, WindowEvent};
use luminance_glfw::GlfwSurface;
use luminance_windowing::WindowOpt;
use pixels_of_life::Core;
use std::time::{Duration, Instant};

const GAME_UPDATE_FREQ_MS: u64 = 500;
const RENDER_TIMER_TIMEOUT_MS: u64 = 500;

fn main() {
  let mut surface = GlfwSurface::new_gl33("Pixels of Life", WindowOpt::default()).unwrap();
  //surface.window.glfw.set_swap_interval(SwapInterval::None);

  let back_buffer = surface.back_buffer().unwrap();
  let window_size = back_buffer.size();
  let mut window_size = [window_size[0] as f32, window_size[1] as f32];
  let mut gen_textures_size = [64., 64.];
  let mut core = Core::new(&mut surface, back_buffer, gen_textures_size).unwrap();

  // UI
  let mut painting = false;
  let mut cursor_pos = None;
  let mut resize = false;
  let mut last_update_instant = Instant::now();
  let mut game_update_freq_ms = GAME_UPDATE_FREQ_MS;
  let mut paused = false;
  let mut edit_grid_res = false;
  let mut resize_grid = false;
  let mut render_timer_timeout = Instant::now();

  'app: loop {
    // handle events
    surface.window.glfw.poll_events();
    for (_, event) in surface.events_rx.try_iter() {
      match event {
        WindowEvent::Key(Key::Escape, _, Action::Release, _) => break 'app,

        WindowEvent::Key(Key::Space, _, Action::Release, _) => {
          paused = !paused;
        }

        WindowEvent::Key(Key::LeftShift, _, Action::Press, _) => {
          edit_grid_res = true;
        }

        WindowEvent::Key(Key::LeftShift, _, Action::Release, _) => {
          edit_grid_res = false;
        }

        WindowEvent::Key(Key::Enter, _, Action::Release, _) => {
          gen_textures_size = window_size;
          resize_grid = true;
        }

        WindowEvent::Key(Key::Backspace, _, Action::Release, _) => {
          core.random_reset().unwrap();
        }

        WindowEvent::FramebufferSize(..) => {
          resize = true;
        }

        WindowEvent::MouseButton(MouseButton::Button1, Action::Press, _) => {
          painting = true;

          // update, if possible
          if let Some(cursor_pos) = cursor_pos {
            let cell_pos = window_to_grid(cursor_pos, window_size, gen_textures_size);
            core.update_cell(1, cell_pos).unwrap();
          }
        }

        WindowEvent::MouseButton(MouseButton::Button1, Action::Release, _) => {
          painting = false;
        }

        WindowEvent::CursorPos(x, y) => {
          let pos = [x as f32, window_size[1] - y as f32];
          cursor_pos = Some(pos);

          if painting {
            let cell_pos = window_to_grid(pos, window_size, gen_textures_size);
            core.update_cell(1, cell_pos).unwrap();
          }
        }

        WindowEvent::Scroll(_, y) => {
          if edit_grid_res {
            resize_grid = true;

            let delta = y as f32 * 5.;
            gen_textures_size = [
              (gen_textures_size[0] + delta).max(0.),
              (gen_textures_size[1] + delta).max(0.),
            ];
          } else {
            game_update_freq_ms =
              (game_update_freq_ms as i64 + y.signum() as i64 * 50).max(5) as u64;
            println!("mutation set every {}ms", game_update_freq_ms);
          }
        }

        _ => (),
      }
    }

    // handle resize
    if resize {
      let back_buffer = surface.back_buffer().unwrap();
      let [w, h] = back_buffer.size();

      window_size = [w as _, h as _];
      core.resize_backbuffer(back_buffer);

      resize = false;
    }

    if resize_grid {
      println!(
        "grid dimension set to {} × {}",
        gen_textures_size[0] as u32, gen_textures_size[1] as u32
      );
      core.resize_grid(&mut surface, gen_textures_size).unwrap();

      resize_grid = false;
    }

    // render the current generation
    let render_timer = Instant::now();
    let render = core.render_gen(&mut surface);

    if render.is_ok() {
      // swap the buffers
      surface.window.swap_buffers();
    } else {
      break 'app;
    }

    // compute the next generation
    if !paused && last_update_instant.elapsed() >= Duration::from_millis(game_update_freq_ms) {
      core.mutate_gen(&mut surface).unwrap();
      core.step_gen();
      last_update_instant = Instant::now();
    }

    let render_time = render_timer.elapsed();
    if render_timer_timeout.elapsed().as_millis() as u64 >= RENDER_TIMER_TIMEOUT_MS {
      println!("{} FPS", (1. / render_time.as_secs_f32()) as u64);
      render_timer_timeout = Instant::now();
    }
  }
}

// Transform between window space into grid space.
fn window_to_grid(pos: [f32; 2], window: [f32; 2], grid: [f32; 2]) -> [f32; 2] {
  let [kw, kh] = [grid[0] / window[0], grid[1] / window[1]];
  [pos[0] * kw, pos[1] * kh]
}
