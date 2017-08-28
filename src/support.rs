use std::ffi::CStr;
use std::mem;
use std::ptr;
use glutin::{self, GlContext};
use gleam::gl;
use std::rc::Rc;

pub struct GlTest {
    gl: Rc<gl::Gl>,
}

pub fn load(gl: Rc<gl::Gl>) -> GlTest {
    let version = gl.get_string(gl::VERSION);

    println!("OpenGL version {}", version);

    let vs = gl.create_shader(gl::VERTEX_SHADER);
    gl.shader_source(vs, &[VS_SRC]);
    gl.compile_shader(vs);
    println!("vs status: {}", gl.get_shader_iv(vs, gl::COMPILE_STATUS));

    let fs = gl.create_shader(gl::FRAGMENT_SHADER);
    gl.shader_source(fs, &[FS_SRC]);
    gl.compile_shader(fs);
    println!("fs status: {}", gl.get_shader_iv(fs, gl::COMPILE_STATUS));

    let program = gl.create_program();
    gl.attach_shader(program, vs);
    gl.attach_shader(program, fs);
    gl.link_program(program);
    gl.use_program(program);

    // println!("vs status: {:?}", log);
    // let log = gl.get_shader_iv(fs, gl::COMPILE_STATUS);
    // println!("fs status: {:?}", log);

    let vb = gl.gen_buffers(1)[0];
    gl.bind_buffer(gl::ARRAY_BUFFER, vb);
    gl.buffer_data_untyped(gl::ARRAY_BUFFER,
                       (VERTEX_DATA.len() * mem::size_of::<f32>()) as gl::types::GLsizeiptr,
                       VERTEX_DATA.as_ptr() as *const _, gl::STATIC_DRAW);

    let vao = gl.gen_vertex_arrays(1)[0];
    gl.bind_vertex_array(vao);

    let pos_attrib = gl.get_attrib_location(program, "position");
    let color_attrib = gl.get_attrib_location(program, "color");
    gl.vertex_attrib_pointer(pos_attrib as gl::types::GLuint, 2, gl::FLOAT, false,
                                5 * mem::size_of::<f32>() as gl::types::GLsizei,
                                0);
    gl.vertex_attrib_pointer(color_attrib as gl::types::GLuint, 3, gl::FLOAT, false,
                                5 * mem::size_of::<f32>() as gl::types::GLsizei,
                                (2 * mem::size_of::<f32>()) as u32);
    gl.enable_vertex_attrib_array(pos_attrib as gl::types::GLuint);
    gl.enable_vertex_attrib_array(color_attrib as gl::types::GLuint);

    GlTest { gl: gl }
}

impl GlTest {
    pub fn draw_frame(&self, color: [f32; 4]) {
        self.gl.clear_color(color[0], color[1], color[2], color[3]);
        self.gl.clear(gl::COLOR_BUFFER_BIT);
        self.gl.draw_arrays(gl::TRIANGLES, 0, 3);

        // let log = self.gl.get_shader_iv(1, gl::COMPILE_STATUS);
        // println!("vs status: {:?}", log);
        // let log = self.gl.get_shader_iv(2, gl::COMPILE_STATUS);
        // println!("fs status: {:?}", log);

        // let log = self.gl.get_shader_info_log(1);
        // println!("vs log: {:?}", log);
        // let log = self.gl.get_shader_info_log(2);
        // println!("fs log: {:?}", log);
    }
}

static VERTEX_DATA: [f32; 15] = [
    -0.5, -0.5, 1.0, 0.0, 0.0,
    0.0, 0.5, 0.0, 1.0, 0.0,
    0.5, -0.5, 0.0, 0.0, 1.0
];

const VS_SRC: &'static [u8] = b"
#version 100
precision mediump float;
attribute vec2 position;
attribute vec3 color;
varying vec3 v_color;
void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    v_color = color;
}";

const FS_SRC: &'static [u8] = b"
#version 100
precision mediump float;
varying vec3 v_color;
void main() {
    gl_FragColor = vec4(v_color, 1.0);
}";
