use lw_apple_sys::metal::Frames;
use ogpu_core::backend as be;
use pxui::{Commands, Gpu, Pass, Renderer, TargetDesc, Ui};
use pxui_sdl::Window;

fn message(error: ogpu_core::error::Error) -> String {
    ogpu_core::error::to_string(&error)
}

pub struct Graphics {
    renderer: Option<Renderer>,
    frames: Frames,
    queue: be::Queue,
    device: be::Device,
    blur_extent: (u32, u32),
}

impl Graphics {
    pub fn open(window: &mut Window, ui: &Ui) -> Result<Self, String> {
        ogpu_core::impl_::select(ogpu_metal::impl_::implementation());
        let driver = ogpu_core::impl_::create_driver().map_err(message)?;
        let device = be::create_device(driver).map_err(message)?;
        let queue = be::create_queue(&device).map_err(message)?;
        let frames = Frames::new(&device, window.token()?)?;
        let native = be::native_leased(&device).map_err(message)?;
        let renderer = Renderer::new(
            ui,
            Gpu {
                size: native.size,
                backend: native.backend,
                instance: native.instance,
                physical_device: native.physical_device,
                device: native.device,
                queue: native.queue,
                queue_family: native.queue_family,
                api_version: native.api_version,
                features: native.features,
                get_instance_proc_addr: native.get_instance_proc_addr,
                pipeline_cache: native.pipeline_cache,
                mtl_device: native.mtl_device,
                wl_display: native.wl_display,
                wl_surface: native.wl_surface,
                metal_layer: native.metal_layer,
                hwnd: native.hwnd,
                owner_token: native.owner_token,
                release_owner: native.release_owner,
            },
            TargetDesc {
                format: 2,
                samples: 1,
            },
        )?;
        Ok(Self {
            renderer: Some(renderer),
            frames,
            queue,
            device,
            blur_extent: (0, 0),
        })
    }

    pub fn draw(&mut self, window: &mut Window, ui: &Ui) -> Result<bool, String> {
        let extent = window.extent();
        let size = (extent.drawable.0 as u32, extent.drawable.1 as u32);
        if size.0 == 0 || size.1 == 0 {
            return Ok(false);
        }
        if self.blur_extent != size {
            self.renderer
                .as_mut()
                .expect("Live graphics renderer")
                .resize(size.0, size.1)
                .map_err(|code| format!("Renderer resize failed ({code})"))?;
            self.blur_extent = size;
        }
        if !self
            .frames
            .begin(extent.drawable.0 as u32, extent.drawable.1 as u32)?
        {
            return Ok(false);
        }
        let renderer = self.renderer.as_mut().expect("Live graphics renderer");
        let commands = self.frames.commands();
        renderer
            .prepare(
                &Commands {
                    size: core::mem::size_of::<Commands>() as u32,
                    mtl_command_buffer: commands,
                    ..Commands::default()
                },
                self.frames.serial(),
                self.frames.completed(),
            )
            .map_err(|code| format!("Renderer prepare failed ({code})"))?;
        let color = ui.theme().tokens.panel;
        let encoder = self.frames.pass([color.r, color.g, color.b])?;
        renderer
            .draw(&Pass {
                size: core::mem::size_of::<Pass>() as u32,
                mtl_command_buffer: commands,
                mtl_render_encoder: encoder,
                width_px: extent.drawable.0 as u32,
                height_px: extent.drawable.1 as u32,
                scale: window.scale() as f32,
                ..Pass::default()
            })
            .map_err(|code| format!("Renderer draw failed ({code})"))?;
        self.frames.submit()?;
        Ok(true)
    }

    pub fn close(mut self) -> Result<(), String> {
        self.frames.wait()
    }
}

impl Drop for Graphics {
    fn drop(&mut self) {
        // Renderer resources must outlive all submitted GPU work, including early error exits.
        let _ = self.frames.wait();
        drop(self.renderer.take());
        let _ = be::destroy_queue(&self.queue);
        let _ = be::destroy_device(&self.device);
    }
}
