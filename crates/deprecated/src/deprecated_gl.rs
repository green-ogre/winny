use std::ffi::c_void;

// pub const GL_COLOR_BUFFER_BIT: c_uint = 0x00004000;
// pub const GL_TEXTURE_2D: GLenum = 0x0DE1;
// pub const GL_RGBA: GLenum = 0x1908;
pub const GL_UNSIGNED_INT_8_8_8_8: GLenum = 0x8035;
pub const GL_READ_FRAMEBUFFER: GLenum = 0x8CA8;
pub const GL_DRAW_FRAMEBUFFER: GLenum = 0x8CA9;
pub const GL_COLOR_ATTACHMENT0: GLenum = 0x8CE0;
// pub const GL_NEAREST: GLenum = 0x2600;
// pub const GL_LINEAR: GLenum = 0x2601;
// pub const GL_TEXTURE_MIN_FILTER: GLenum = 0x2801;
// pub const GL_TEXTURE_MAG_FILTER: GLenum = 0x2800;

pub type c_int = i32;
pub type c_uint = u32;
pub type c_float = f32;

pub type GLbitfield = c_uint;
pub type GLfloat = c_float;
pub type GLint = i32;
pub type GLuint = u32;
pub type GLsizei = i32;
pub type GLenum = u32;
pub type GLchar = i8;

pub type glClear_t = Option<unsafe extern "system" fn(mask: GLbitfield)>;

pub type glClearColor_t =
    Option<unsafe extern "system" fn(red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat)>;

pub type glViewport_t =
    Option<unsafe extern "system" fn(x: GLint, y: GLint, width: GLsizei, height: GLsizei)>;

pub type glGenTextures_t = Option<unsafe extern "system" fn(n: GLsizei, textures: *mut GLuint)>;

pub type glBindTexture_t = Option<unsafe extern "system" fn(target: GLenum, texture: GLuint)>;

pub type glTexImage2D_t = Option<
    unsafe extern "system" fn(
        target: GLenum,
        level: GLint,
        internal_format: GLint,
        width: GLsizei,
        height: GLsizei,
        border: GLint,
        format: GLenum,
        _type: GLenum,
        data: *mut c_void,
    ),
>;

pub type glGenFrameBuffers_t = Option<unsafe extern "system" fn(n: GLsizei, ids: *mut GLuint)>;

pub type glBindFramebuffer_t =
    Option<unsafe extern "system" fn(target: GLenum, framebuffer: GLuint)>;

pub type glFrameBufferTexture2D_t = Option<
    unsafe extern "system" fn(
        target: GLenum,
        attachment: GLenum,
        textarget: GLenum,
        texture: GLuint,
        level: GLint,
    ),
>;

pub type glBlitFramebuffer_t = Option<
    unsafe extern "system" fn(
        src_x0: GLint,
        src_y0: GLint,
        src_x1: GLint,
        src_y1: GLint,
        dst_x0: GLint,
        dst_y0: GLint,
        dst_x1: GLint,
        dst_y1: GLint,
        mask: GLbitfield,
        filter: GLenum,
    ),
>;

pub type glGetError_t = Option<unsafe extern "system" fn() -> GLenum>;

pub type glTexParameteri_t =
    Option<unsafe extern "system" fn(target: GLenum, pname: GLenum, param: GLint) -> GLenum>;

pub type glGetDebugMessageLog_t = Option<
    unsafe extern "system" fn(
        count: GLuint,
        bufSize: GLsizei,
        sources: *mut GLenum,
        types: *mut GLenum,
        ids: *mut GLuint,
        severities: *mut GLenum,
        lengths: *mut GLsizei,
        messageLog: *mut GLchar,
    ) -> GLuint,
>;
