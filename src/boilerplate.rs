/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use gleam::gl;
use glutin;
use std::env;
use std::path::PathBuf;
use webrender;
use webrender::api::*;
use webrender::renderer::{PROFILER_DBG, RENDER_TARGET_DBG, TEXTURE_CACHE_DBG};
use webrender::renderer::ExternalImageHandler;
// use support;

use glutin::GlContext;

struct Notifier {
    loop_proxy: glutin::EventsLoopProxy,
}

impl Notifier {
    fn new(loop_proxy: glutin::EventsLoopProxy)-> Notifier {
        Notifier {
            loop_proxy,
        }
    }
}

impl RenderNotifier for Notifier {
    fn new_frame_ready(&mut self) {
        #[cfg(not(target_os = "android"))]
        self.loop_proxy.wakeup().unwrap(); // TODO maybe don't unwrap
    }

    fn new_scroll_frame_ready(&mut self, _composite_needed: bool) {
        #[cfg(not(target_os = "android"))]
        self.loop_proxy.wakeup().unwrap(); // TODO maybe don't unwrap
    }
}

pub trait HandyDandyRectBuilder {
    fn to(&self, x2: i32, y2: i32) -> LayoutRect;
    fn by(&self, w: i32, h: i32) -> LayoutRect;
}
// Allows doing `(x, y).to(x2, y2)` or `(x, y).by(width, height)` with i32
// values to build a f32 LayoutRect
impl HandyDandyRectBuilder for (i32, i32) {
    fn to(&self, x2: i32, y2: i32) -> LayoutRect {
        LayoutRect::new(LayoutPoint::new(self.0 as f32, self.1 as f32),
                        LayoutSize::new((x2 - self.0) as f32, (y2 - self.1) as f32))
    }

    fn by(&self, w: i32, h: i32) -> LayoutRect {
        LayoutRect::new(LayoutPoint::new(self.0 as f32, self.1 as f32),
                        LayoutSize::new(w as f32, h as f32))
    }
}

pub trait Example {
    fn render(&mut self,
              api: &RenderApi,
              builder: &mut DisplayListBuilder,
              resources: &mut ResourceUpdates,
              layout_size: LayoutSize,
              pipeline_id: PipelineId,
              document_id: DocumentId);
    fn on_event(&mut self,
                event: glutin::WindowEvent,
                api: &RenderApi,
                document_id: DocumentId) -> bool;
    fn get_external_image_handler(&self) -> Option<Box<ExternalImageHandler>> {
        None
    }
}

pub fn main_wrapper(example: &mut Example,
                    options: Option<webrender::RendererOptions>)
{
    let args: Vec<String> = env::args().collect();
    let res_path = if args.len() > 1 {
        Some(PathBuf::from(&args[1]))
    } else {
        None
    };

    let mut events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new()
        .with_multitouch()
        .with_visibility(false)
        .with_title("WebRender Playground");
    let context = glutin::ContextBuilder::new()
        .with_gl(glutin::GlRequest::GlThenGles {
            opengl_version: (3, 2),
            opengles_version: (3, 0)
        });
    let window = glutin::GlWindow::new(window_builder, context, &events_loop).unwrap();

    unsafe { window.make_current().ok() };

    println!("Pixel format of the window's GL context: {:?}", window.get_pixel_format());

    let gl = match gl::GlType::default() {
        gl::GlType::Gl => unsafe { gl::GlFns::load_with(|symbol| window.get_proc_address(symbol) as *const _) },
        gl::GlType::Gles => unsafe { gl::GlesFns::load_with(|symbol| window.get_proc_address(symbol) as *const _) },
    };
    // let gl = unsafe { gl::GlesFns::load_with(|symbol| window.get_proc_address(symbol) as *const _) };

    println!("OpenGL version {}", gl.get_string(gl::VERSION));
    println!("Shader resource path: {:?}", res_path);
    // let sgl = gl.clone();

    let (mut width, mut height) = window.get_inner_size_pixels().unwrap();

    let opts = webrender::RendererOptions {
        resource_override_path: res_path,
        debug: true,
        precache_shaders: false,
        device_pixel_ratio: window.hidpi_factor(),
        .. options.unwrap_or(webrender::RendererOptions::default())
    };

    let size = DeviceUintSize::new(width, height);
    let (mut renderer, sender) = webrender::renderer::Renderer::new(gl, opts).unwrap();
    let api = sender.create_api();
    let document_id = api.add_document(size);

    let notifier = Box::new(Notifier::new(events_loop.create_proxy()));
    renderer.set_render_notifier(notifier);

    if let Some(external_image_handler) = example.get_external_image_handler() {
        renderer.set_external_image_handler(external_image_handler);
    }

    let epoch = Epoch(0);
    let root_background_color = ColorF::new(0.3, 0.0, 0.0, 1.0);

    let pipeline_id = PipelineId(0, 0);
    let layout_size = LayoutSize::new(width as f32, height as f32);
    let mut builder = DisplayListBuilder::new(pipeline_id, layout_size);
    let mut resources = ResourceUpdates::new();

    example.render(&api, &mut builder, &mut resources, layout_size, pipeline_id, document_id);
    api.set_display_list(
        document_id,
        epoch,
        Some(root_background_color),
        LayoutSize::new(width as f32, height as f32),
        builder.finalize(),
        true,
        resources
    );
    api.set_root_pipeline(document_id, pipeline_id);
    api.generate_frame(document_id, None);

    // let gl_test = support::load(sgl);
    window.show();

    events_loop.run_forever(|event| {
        println!("{:?}", event);
        match event {
            glutin::Event::WindowEvent { event, .. } => {
                match event {
                    glutin::WindowEvent::Resized(w, h) => {
                        window.resize(w, h);
                        width = w;
                        height = h;
                        let size = DeviceUintSize::new(width, height);
                        let rect = DeviceUintRect::new(DeviceUintPoint::zero(), size);
                        api.set_window_parameters(document_id, size, rect);
                    },
                    glutin::WindowEvent::Closed |
                    glutin::WindowEvent::KeyboardInput {
                        input: glutin::KeyboardInput {virtual_keycode: Some(glutin::VirtualKeyCode::Escape), .. }, ..
                    } => return glutin::ControlFlow::Break,
                    /*
                    glutin::WindowEvent::KeyboardInput(glutin::ElementState::Pressed,
                                                 _, Some(glutin::VirtualKeyCode::P)) => {
                        let mut flags = renderer.get_debug_flags();
                        flags.toggle(PROFILER_DBG);
                        renderer.set_debug_flags(flags);
                    }
                    glutin::WindowEvent::KeyboardInput(glutin::ElementState::Pressed,
                                                 _, Some(glutin::VirtualKeyCode::O)) => {
                        let mut flags = renderer.get_debug_flags();
                        flags.toggle(RENDER_TARGET_DBG);
                        renderer.set_debug_flags(flags);
                    }
                    glutin::WindowEvent::KeyboardInput(glutin::ElementState::Pressed,
                                                 _, Some(glutin::VirtualKeyCode::I)) => {
                        let mut flags = renderer.get_debug_flags();
                        flags.toggle(TEXTURE_CACHE_DBG);
                        renderer.set_debug_flags(flags);
                    }
                    glutin::WindowEvent::KeyboardInput(glutin::ElementState::Pressed,
                                                 _, Some(glutin::VirtualKeyCode::M)) => {
                        api.notify_memory_pressure();
                    }
                    */
                    _ => (),
                }
                if example.on_event(event, &api, document_id) {
                    let mut builder = DisplayListBuilder::new(pipeline_id, layout_size);
                    let mut resources = ResourceUpdates::new();

                    let layout_size = LayoutSize::new(width as f32, height as f32);
                    example.render(&api, &mut builder, &mut resources, layout_size, pipeline_id, document_id);
                    api.set_display_list(
                        document_id,
                        epoch,
                        Some(root_background_color),
                        layout_size,
                        builder.finalize(),
                        true,
                        resources
                    );
                    api.generate_frame(document_id, None);
                }
            },
            _ => (),
        }

        renderer.update();
        renderer.render(DeviceUintSize::new(width, height));
        // gl_test.draw_frame([0.0, 1.0, 0.0, 1.0]);
        window.swap_buffers().ok();
        glutin::ControlFlow::Continue
    });

    renderer.deinit();
}
/*
extern crate glutin;

mod support;

use glutin::GlContext;

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_title("A fantastic window!");
    let context = glutin::ContextBuilder::new();
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    let _ = unsafe { gl_window.make_current() };

    println!("Pixel format of the window's GL context: {:?}", gl_window.get_pixel_format());

    let gl = support::load(&gl_window);

    events_loop.run_forever(|event| {
        println!("{:?}", event);
        match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::Closed => return glutin::ControlFlow::Break,
                glutin::WindowEvent::Resized(w, h) => gl_window.resize(w, h),
                _ => (),
            },
            _ => ()
        }

        gl.draw_frame([0.0, 1.0, 0.0, 1.0]);
        let _ = gl_window.swap_buffers();
        glutin::ControlFlow::Continue
    });
}

*/
