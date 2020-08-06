use luminance_derive::UniformInterface;
use luminance_front::context::GraphicsContext;
use luminance_front::framebuffer::{Framebuffer, FramebufferError};
use luminance_front::pipeline::{PipelineError, PipelineState, Render, TextureBinding};
use luminance_front::pixel::{Unsigned, R8UI};
use luminance_front::render_state::RenderState;
use luminance_front::shader::{Program, ProgramError, Uniform, UniformInterface};
use luminance_front::tess::{Mode, Tess, TessError};
use luminance_front::texture::{Dim2, GenMipmaps, Sampler, TextureError};
use luminance_front::Backend;
use rand::{thread_rng, Rng};
use std::fmt;

/// All possible errors.
#[derive(Debug)]
pub enum CoreError {
  FramebufferError(FramebufferError),
  TessError(TessError),
  ProgramError(ProgramError),
  PipelineError(PipelineError),
  TextureError(TextureError),
}

impl fmt::Display for CoreError {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    match *self {
      CoreError::FramebufferError(ref e) => write!(f, "framebuffer error: {}", e),
      CoreError::TessError(ref e) => write!(f, "tessellation error: {}", e),
      CoreError::ProgramError(ref e) => write!(f, "shader program error: {}", e),
      CoreError::PipelineError(ref e) => write!(f, "pipeline error: {}", e),
      CoreError::TextureError(ref e) => write!(f, "texture error: {}", e),
    }
  }
}

impl From<FramebufferError> for CoreError {
  fn from(e: FramebufferError) -> Self {
    CoreError::FramebufferError(e)
  }
}

impl From<TessError> for CoreError {
  fn from(e: TessError) -> Self {
    CoreError::TessError(e)
  }
}

impl From<ProgramError> for CoreError {
  fn from(e: ProgramError) -> Self {
    CoreError::ProgramError(e)
  }
}

impl From<PipelineError> for CoreError {
  fn from(e: PipelineError) -> Self {
    CoreError::PipelineError(e)
  }
}

impl From<TextureError> for CoreError {
  fn from(e: TextureError) -> Self {
    CoreError::TextureError(e)
  }
}

#[derive(Debug, UniformInterface)]
pub struct MutateShaderInterface {
  #[uniform(unbound)]
  current_gen_texture: Uniform<TextureBinding<Dim2, Unsigned>>,
}

#[derive(Debug, UniformInterface)]
pub struct CopyShaderInterface {
  #[uniform(unbound)]
  source: Uniform<TextureBinding<Dim2, Unsigned>>,
  #[uniform(unbound)]
  scale_ratio: Uniform<[f32; 2]>,
}

// Shader sources.
const FULLSCREEN_QUAD_SHADER_VS: &str = include_str!("shaders/fullscreen_quad.vs.glsl");
const MUTATE_SHADER_FS: &str = include_str!("shaders/mutate.fs.glsl");
const COPY_SHADER_FS: &str = include_str!("shaders/copy.fs.glsl");

/// Core logic of the application.
pub struct Core {
  back_buffer: Framebuffer<Dim2, (), ()>,
  gen_framebuffers: [Framebuffer<Dim2, R8UI, ()>; 2],
  current_gen: u8,
  fullscreen_quad: Tess<()>,
  mutate_shader: Program<(), (), MutateShaderInterface>,
  copy_shader: Program<(), (), CopyShaderInterface>,
}

impl Core {
  pub fn new(
    surface: &mut impl GraphicsContext<Backend = Backend>,
    back_buffer: Framebuffer<Dim2, (), ()>,
    gen_textures_size: [f32; 2],
  ) -> Result<Self, CoreError> {
    let gen_framebuffers = Self::create_gen_framebuffers(surface, gen_textures_size)?;
    let current_gen = 0;
    let fullscreen_quad = Self::create_fullscreen_quad(surface)?;
    let mutate_shader = Self::create_shader(surface, FULLSCREEN_QUAD_SHADER_VS, MUTATE_SHADER_FS)?;
    let copy_shader = Self::create_shader(surface, FULLSCREEN_QUAD_SHADER_VS, COPY_SHADER_FS)?;

    Ok(Core {
      back_buffer,
      gen_framebuffers,
      current_gen,
      fullscreen_quad,
      mutate_shader,
      copy_shader,
    })
  }

  pub fn resize_backbuffer(&mut self, back_buffer: Framebuffer<Dim2, (), ()>) {
    self.back_buffer = back_buffer;
  }

  pub fn resize_grid(
    &mut self,
    surface: &mut impl GraphicsContext<Backend = Backend>,
    size: [f32; 2],
  ) -> Result<(), CoreError> {
    self.gen_framebuffers = Self::create_gen_framebuffers(surface, size)?;
    Ok(())
  }

  /// Mutate current generation and output the next generation.
  pub fn mutate_gen(
    &mut self,
    surface: &mut impl GraphicsContext<Backend = Backend>,
  ) -> Result<(), CoreError> {
    let current_gen_framebuffer;
    let next_gen_framebuffer;
    let [ref mut fb_a, ref mut fb_b] = self.gen_framebuffers;

    if self.current_gen == 0 {
      current_gen_framebuffer = fb_a;
      next_gen_framebuffer = fb_b;
    } else {
      next_gen_framebuffer = fb_a;
      current_gen_framebuffer = fb_b;
    }

    let current_gen_texture = current_gen_framebuffer.color_slot();

    let mutate_shader = &mut self.mutate_shader;
    let fullscreen_quad = &self.fullscreen_quad;

    surface
      .new_pipeline_gate()
      .pipeline(
        next_gen_framebuffer,
        &PipelineState::default().enable_clear_color(false),
        |pipeline, mut shd_gate| {
          let current_gen_texture = pipeline.bind_texture(current_gen_texture)?;

          shd_gate.shade(mutate_shader, |mut iface, uni, mut rdr_gate| {
            iface.set(&uni.current_gen_texture, current_gen_texture.binding());

            rdr_gate.render(&RenderState::default(), |mut tess_gt| {
              tess_gt.render(fullscreen_quad)
            })
          })
        },
      )
      .assume()
      .into_result()?;

    Ok(())
  }

  pub fn step_gen(&mut self) {
    self.current_gen = 1 - self.current_gen;
  }

  pub fn render_gen(
    &mut self,
    surface: &mut impl GraphicsContext<Backend = Backend>,
  ) -> Render<PipelineError> {
    let scale_ratio = self.window_to_grid_scale_ratio();
    let current_gen_texture = self.gen_framebuffers[self.current_gen as usize].color_slot();
    let copy_shader = &mut self.copy_shader;
    let fullscreen_quad = &self.fullscreen_quad;

    surface.new_pipeline_gate().pipeline(
      &self.back_buffer,
      &PipelineState::default().set_clear_color([0.5, 1., 0.5, 1.]),
      |pipeline, mut shd_gate| {
        let source = pipeline.bind_texture(current_gen_texture)?;

        shd_gate.shade(copy_shader, |mut iface, uni, mut rdr_gate| {
          iface.set(&uni.source, source.binding());
          iface.set(&uni.scale_ratio, scale_ratio);

          rdr_gate.render(&RenderState::default(), |mut tess_gt| {
            tess_gt.render(fullscreen_quad)
          })
        })
      },
    )
  }

  pub fn update_cell(&mut self, cell: u8, pos: [f32; 2]) -> Result<(), CoreError> {
    let current_gen_texture = self.gen_framebuffers[self.current_gen as usize].color_slot();
    Ok(current_gen_texture.upload_part(
      GenMipmaps::No,
      [pos[0] as _, pos[1] as _],
      [1, 1],
      &[cell],
    )?)
  }

  // Get the window to grid scale ratio.
  pub fn window_to_grid_scale_ratio(&self) -> [f32; 2] {
    let window = self.back_buffer.size();
    let grid = self.gen_framebuffers[0].size();

    [
      grid[0] as f32 / window[0] as f32,
      grid[1] as f32 / window[1] as f32,
    ]
  }

  // Reset the grid with random values.
  pub fn random_reset(&mut self) -> Result<(), CoreError> {
    let fb = &self.gen_framebuffers[self.current_gen as usize];
    let [w, h] = fb.size();
    let mut rng = thread_rng();

    let values = (0..w * h)
      .map(|_| rng.gen_bool(0.06) as u8)
      .collect::<Vec<_>>();

    Ok(
      self.gen_framebuffers[self.current_gen as usize]
        .color_slot()
        .upload_part(GenMipmaps::No, [0, 0], [w, h], &values[..])?,
    )
  }

  fn create_gen_framebuffers(
    surface: &mut impl GraphicsContext<Backend = Backend>,
    gen_textures_size: [f32; 2],
  ) -> Result<[Framebuffer<Dim2, R8UI, ()>; 2], CoreError> {
    let size = [gen_textures_size[0] as u32, gen_textures_size[1] as u32];

    let mut fbs: [Framebuffer<Dim2, R8UI, ()>; 2] = [
      surface.new_framebuffer(size, 0, Sampler::default())?,
      surface.new_framebuffer(size, 0, Sampler::default())?,
    ];

    for fb in &mut fbs {
      fb.color_slot().clear(GenMipmaps::No, 0)?;
    }

    Ok(fbs)
  }

  fn create_fullscreen_quad(
    surface: &mut impl GraphicsContext<Backend = Backend>,
  ) -> Result<Tess<()>, CoreError> {
    Ok(
      surface
        .new_tess()
        .set_mode(Mode::TriangleFan)
        .set_vertex_nb(4)
        .build()?,
    )
  }

  fn create_shader<U>(
    surface: &mut impl GraphicsContext<Backend = Backend>,
    vs: &str,
    fs: &str,
  ) -> Result<Program<(), (), U>, CoreError>
  where
    U: UniformInterface<Backend>,
  {
    let shader = surface
      .new_shader_program()
      .from_strings(vs, None, None, fs)?
      .ignore_warnings();

    Ok(shader)
  }
}
