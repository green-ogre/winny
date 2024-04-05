#![allow(unused)]

use std::ffi::c_char;
use std::io::Read;
use std::time::SystemTime;

use ecs::World;
use logging::*;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Graphics::OpenGL::*;
use windows::Win32::Media::Audio::XAudio2::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::System::Performance::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::gfx::bitmap::*;
use crate::gl::*;
use crate::input::*;

pub const WGL_DRAW_TO_WINDOW_ARB: c_int = 0x2001;
pub const WGL_SUPPORT_OPENGL_ARB: c_int = 0x2010;
pub const WGL_DOUBLE_BUFFER_ARB: c_int = 0x2011;
pub const WGL_PIXEL_TYPE_ARB: c_int = 0x2013;
pub const WGL_COLOR_BITS_ARB: c_int = 0x2014;
pub const WGL_DEPTH_BITS_ARB: c_int = 0x2022;
pub const WGL_STENCIL_BITS_ARB: c_int = 0x2023;
pub const WGL_TYPE_RGBA_ARB: c_int = 0x202B;
pub const WGL_FRAMEBUFFER_SRGB_CAPABLE_EXT: c_int = 0x20A9;
pub const WGL_SAMPLE_BUFFERS_ARB: c_int = 0x2041;
pub const WGL_CONTEXT_MAJOR_VERSION_ARB: c_int = 0x2091;
pub const WGL_CONTEXT_MINOR_VERSION_ARB: c_int = 0x2092;
pub const WGL_CONTEXT_FLAGS_ARB: c_int = 0x2094;
pub const WGL_CONTEXT_PROFILE_MASK_ARB: c_int = 0x9126;
pub const WGL_CONTEXT_DEBUG_BIT_ARB: c_int = 0x0001;
pub const WGL_CONTEXT_FORWARD_COMPATIBLE_BIT_ARB: c_int = 0x0002;
pub const WGL_CONTEXT_CORE_PROFILE_BIT_ARB: c_int = 0x00000001;

fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

#[macro_export]
macro_rules! c_str {
    ($text:expr) => {{
        concat!($text, '\0').as_bytes()
    }};
}

unsafe fn gather_null_terminated_bytes(mut p: *const u8) -> Vec<u8> {
    let mut v = vec![];
    while *p != 0 {
        v.push(*p);
        p = p.add(1);
    }
    v
}

fn lossy_c_str_to_string(bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    }
}

#[allow(non_camel_case_types)]
type Win32WindowCallback_t =
    unsafe extern "system" fn(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;

unsafe extern "system" fn _DefWindowProcW(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn win32_window_callback(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // println!("LOOP: {}", msg);
    match msg {
        WM_NCCREATE => {
            let wnd_lparam: *mut CREATESTRUCTW = lparam.0 as *mut CREATESTRUCTW;
            if wnd_lparam.is_null() {
                return LRESULT(0);
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*wnd_lparam).lpCreateParams as isize);
            return LRESULT(1);
        }
        WM_DESTROY => {
            // There is no good reason to clean up here yet
            PostQuitMessage(0);
        }
        WM_CLOSE => {
            PostQuitMessage(0);
        }
        WM_SIZE => {
            let wnd_data = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;
            assert!(!wnd_data.is_null());

            let width = lparam.0 & 0x0000ffff;
            let height = lparam.0 >> 16;

            (*wnd_data).tex_data.width = width as usize;
            (*wnd_data).tex_data.height = height as usize;
        }
        WM_PAINT => {
            // let wnd_data = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;
            // assert!(!wnd_data.is_null());

            // TODO: might cause flickering
            // unsafe { win32_render(wnd_data) };
            // SwapBuffers((*wnd_data).hdc);
        }
        WM_KEYDOWN => {
            let input = usize_to_keycode(wparam.0);

            let input = KeyInput {
                vk: input,
                state: KeyState::Pressed,
            };

            let wnd_data = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;
            assert!(!wnd_data.is_null());

            (*(*wnd_data).input_buf).push(input);
        }
        WM_KEYUP => {
            let input = usize_to_keycode(wparam.0);

            let input = KeyInput {
                vk: input,
                state: KeyState::Released,
            };

            let wnd_data = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;
            assert!(!wnd_data.is_null());

            (*(*wnd_data).input_buf).push(input);
        }
        _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
    }
    LRESULT(0)
}

fn usize_to_keycode(input: usize) -> KeyCode {
    match VIRTUAL_KEY(input as u16) {
        VK_ESCAPE => KeyCode::Escape,
        VK_1 => KeyCode::Key1,
        VK_2 => KeyCode::Key2,
        VK_W => KeyCode::W,
        VK_S => KeyCode::S,
        VK_A => KeyCode::A,
        VK_D => KeyCode::D,
        VK_H => KeyCode::H,
        VK_J => KeyCode::J,
        VK_K => KeyCode::K,
        VK_L => KeyCode::L,
        VK_E => KeyCode::E,
        VK_I => KeyCode::I,
        _ => KeyCode::Unknown,
    }
}

fn win32_register_window_class(
    class_name: Vec<u16>,
    callback: Option<Win32WindowCallback_t>,
) -> HINSTANCE {
    let h_instance = unsafe { GetModuleHandleW(PCWSTR::null()).unwrap() };

    let mut wc = WNDCLASSW::default();
    wc.lpfnWndProc = match callback {
        Some(callback) => Some(callback),
        None => Some(_DefWindowProcW),
    };
    wc.hInstance = h_instance.into();
    wc.lpszClassName = PCWSTR(class_name.as_ptr());
    // TODO: fix cursor
    // wc.hCursor = unsafe { LoadCursorW(HINSTANCE::default(), IDC_ARROW) };

    let result = unsafe { RegisterClassW(&wc) };
    if result == 0 {
        let error = unsafe { GetLastError() };
        panic!("Failed to register window: {:?}", error);
    }

    HINSTANCE(h_instance.0)
}

fn win32_create_window(
    width: i32,
    height: i32,
    class: Vec<u16>,
    w_name: Vec<u16>,
    h_instance: HINSTANCE,
    style: WINDOW_STYLE,
    lparam: Option<*const std::ffi::c_void>,
) -> HWND {
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class.as_ptr()),
            PCWSTR(w_name.as_ptr()),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            HWND::default(),
            HMENU::default(),
            h_instance,
            lparam,
        )
    };
    if hwnd.0 == 0 {
        panic!("Failed to create a window.");
    }

    hwnd
}

// TODO: add errors and shit
fn wgl_get_proc_address(func_name: &[u8]) -> Option<PROC> {
    let Some(b'\0') = func_name.last() else {
        return None;
    };

    let proc = unsafe { wglGetProcAddress(PCSTR(func_name.as_ptr())) };
    match proc {
        // TODO: Check this shit
        // 0 | 1 | 2 | 3 | std::usize::MAX => {
        //     let error = unsafe { GetLastError() };
        //     println!("get proc error: {}", error);
        //     return None;
        // },
        _ => Some(proc),
    }
}

fn gl_get_proc_address(h_module: HMODULE, func_name: &[u8]) -> FARPROC {
    let Some(b'\0') = func_name.last() else {
        return None;
    };

    unsafe { GetProcAddress(h_module, PCSTR(func_name.as_ptr())) }
}

unsafe fn wgl_get_extension_string_arb(hdc: HDC) -> String {
    // what is this shananagins
    let wgl_get_extension_string_arb: Option<unsafe extern "system" fn(HDC) -> *const c_char> = unsafe {
        core::mem::transmute(wgl_get_proc_address(c_str!("wglGetExtensionsStringARB")).unwrap())
    };
    let mut extension_string: *const u8 =
        unsafe { (wgl_get_extension_string_arb.unwrap())(hdc) }.cast();
    if extension_string.is_null() {
        let error = unsafe { GetLastError() };
        panic!("Failed to get extension strings: {:?}", error);
    }
    lossy_c_str_to_string(gather_null_terminated_bytes(extension_string))
}

#[allow(non_camel_case_types)]
type wglChoosePixelFormatARB_t = Option<
    unsafe extern "system" fn(
        hdc: HDC,
        piAttribIList: *const i32,
        pfAttribFList: *const f32,
        nMaxFormats: u32,
        piFormats: *mut i32,
        nNumFormats: *mut u32,
    ) -> BOOL,
>;

#[allow(non_camel_case_types)]
type wglCreateContextAttribsARB_t = Option<
    unsafe extern "system" fn(hdc: HDC, hShareContext: HGLRC, attribList: *const i32) -> HGLRC,
>;

#[allow(non_camel_case_types)]
type wglSwapIntervalEXT_t = Option<unsafe extern "system" fn(interval: i32) -> BOOL>;

fn win32_grab_wgl_pointers() -> (
    Vec<String>,
    wglChoosePixelFormatARB_t,
    wglCreateContextAttribsARB_t,
    wglSwapIntervalEXT_t,
) {
    // gl stuff
    let class = wide_null("InitWindowClass");
    let w_name = wide_null("INIT");
    let h_instance = win32_register_window_class(class.clone(), None);
    let init_hwnd = win32_create_window(
        100,
        100,
        class.clone(),
        w_name,
        h_instance,
        WS_OVERLAPPEDWINDOW,
        None,
    );

    let pfd = PIXELFORMATDESCRIPTOR {
        dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
        iPixelType: PFD_TYPE_RGBA,
        cColorBits: 32,
        cDepthBits: 24,
        cStencilBits: 8,
        iLayerType: PFD_MAIN_PLANE.0 as u8,
        ..Default::default()
    };

    let init_hdc = unsafe { GetDC(init_hwnd) };
    let pfi = unsafe { ChoosePixelFormat(init_hdc, &pfd) };
    if pfi == 0 {
        let error = unsafe { GetLastError() };
        panic!("Failed to choose a pixel format: {:?}", error);
    }
    unsafe { SetPixelFormat(init_hdc, pfi, &pfd) };

    let hglrc = match unsafe { wglCreateContext(init_hdc) } {
        Ok(v) => v,
        Err(_) => {
            let error = unsafe { GetLastError() };
            panic!("Failed to create context: {:?}", error);
        }
    };

    let result = unsafe { wglMakeCurrent(init_hdc, hglrc) };
    if !result.is_ok() {
        let error = unsafe { GetLastError() };
        panic!("Failed to make context current: {:?}", error);
    }

    let ext_str = unsafe { wgl_get_extension_string_arb(init_hdc) };
    let extensions: Vec<_> = ext_str
        .split(' ')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    // println!("> Extension Strings: {:?}", extensions);

    // Now we grab some extension functions

    let wgl_choose_pixel_format_arb: wglChoosePixelFormatARB_t = unsafe {
        core::mem::transmute(wgl_get_proc_address(c_str!("wglChoosePixelFormatARB")).unwrap())
    };

    let wgl_create_context_attribs_arb: wglCreateContextAttribsARB_t = unsafe {
        core::mem::transmute(wgl_get_proc_address(c_str!("wglCreateContextAttribsARB")).unwrap())
    };

    let wgl_swap_interval_ext: wglSwapIntervalEXT_t = unsafe {
        core::mem::transmute(wgl_get_proc_address(c_str!("wglSwapIntervalEXT")).unwrap())
    };
    assert!(wgl_choose_pixel_format_arb.is_some());
    assert!(wgl_create_context_attribs_arb.is_some());
    assert!(wgl_swap_interval_ext.is_some());

    let result = unsafe { wglMakeCurrent(HDC::default(), HGLRC::default()) };
    if !result.is_ok() {
        let error = unsafe { GetLastError() };
        panic!("{:?}", error);
    }

    let result = unsafe { wglDeleteContext(hglrc) };
    if !result.is_ok() {
        let error = unsafe { GetLastError() };
        panic!("{:?}", error);
    }

    // clean up the init stuff
    unsafe { ReleaseDC(init_hwnd, init_hdc) };
    unsafe { DestroyWindow(init_hwnd) };
    unsafe { UnregisterClassW(PCWSTR(class.as_ptr()), h_instance) };

    (
        extensions,
        wgl_choose_pixel_format_arb,
        wgl_create_context_attribs_arb,
        wgl_swap_interval_ext,
    )
}

struct GL {
    gl_get_debug_message_log: glGetDebugMessageLog_t,
    gl_tex_parameteri: glTexParameteri_t,
    gl_get_error: glGetError_t,
    gl_clear: glClear_t,
    gl_clear_color: glClearColor_t,
    gl_viewport: glViewport_t,
    gl_gen_textures: glGenTextures_t,
    gl_bind_texture: glBindTexture_t,
    gl_tex_image_2d: glTexImage2D_t,
    gl_gen_framebuffers: glGenFrameBuffers_t,
    gl_bind_framebuffer: glBindFramebuffer_t,
    gl_framebuffer_texture_2d: glFrameBufferTexture2D_t,
    gl_blit_framebuffer: glBlitFramebuffer_t,
}

impl GL {
    unsafe fn bind_functions(opengl32: HMODULE, wnd_data: *mut WindowData) -> Self {
        GL {
            gl_get_debug_message_log: std::mem::transmute(
                wgl_get_proc_address(c_str!("glGetDebugMessageLog")).unwrap(),
            ),
            gl_tex_parameteri: std::mem::transmute(
                gl_get_proc_address(opengl32, c_str!("glTexParameteri")).unwrap(),
            ),
            gl_get_error: std::mem::transmute(
                gl_get_proc_address(opengl32, c_str!("glGetError")).unwrap(),
            ),
            gl_clear: std::mem::transmute(
                gl_get_proc_address(opengl32, c_str!("glClear")).unwrap(),
            ),
            gl_clear_color: std::mem::transmute(
                gl_get_proc_address(opengl32, c_str!("glClearColor")).unwrap(),
            ),
            gl_viewport: std::mem::transmute(
                gl_get_proc_address(opengl32, c_str!("glViewport")).unwrap(),
            ),
            gl_gen_textures: std::mem::transmute(
                gl_get_proc_address(opengl32, c_str!("glGenTextures")).unwrap(),
            ),
            gl_bind_texture: std::mem::transmute(
                gl_get_proc_address(opengl32, c_str!("glBindTexture")).unwrap(),
            ),
            gl_tex_image_2d: std::mem::transmute(
                gl_get_proc_address(opengl32, c_str!("glTexImage2D")).unwrap(),
            ),
            gl_gen_framebuffers: std::mem::transmute(
                wgl_get_proc_address(c_str!("glGenFramebuffers")).unwrap(),
            ),
            gl_bind_framebuffer: std::mem::transmute(
                wgl_get_proc_address(c_str!("glBindFramebuffer")).unwrap(),
            ),
            gl_framebuffer_texture_2d: std::mem::transmute(
                wgl_get_proc_address(c_str!("glFramebufferTexture2D")).unwrap(),
            ),
            gl_blit_framebuffer: std::mem::transmute(
                wgl_get_proc_address(c_str!("glBlitFramebuffer")).unwrap(),
            ),
        }
    }
}

struct WindowData {
    hwnd: HWND,
    hdc: HDC,
    hglrc: HGLRC,
    gl: GL,
    tex_data: TextureData,
    input_buf: *mut InputBuffer,
}

struct TextureData {
    width: usize,
    height: usize,
}

impl Default for WindowData {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

unsafe fn wgl_choose_pixel_format(
    f: wglChoosePixelFormatARB_t,
    hdc: HDC,
    extensions: &Vec<String>,
) -> i32 {
    // let float_attribs: &[[FLOAT; 2]] = &[];
    let mut int_attribs = vec![
        [WGL_DRAW_TO_WINDOW_ARB, true as _],
        [WGL_SUPPORT_OPENGL_ARB, true as _],
        [WGL_DOUBLE_BUFFER_ARB, true as _],
        [WGL_PIXEL_TYPE_ARB, WGL_TYPE_RGBA_ARB],
        [WGL_COLOR_BITS_ARB, 32],
        [WGL_DEPTH_BITS_ARB, 24],
        [WGL_STENCIL_BITS_ARB, 8],
    ];
    if extensions.iter().any(|s| s == "WGL_EXT_framebuffer_sRGB") {
        int_attribs.push([WGL_FRAMEBUFFER_SRGB_CAPABLE_EXT, true as _]);
    }
    if extensions.iter().any(|s| s == "WGL_ARB_multisample") {
        int_attribs.push([WGL_SAMPLE_BUFFERS_ARB, 1]);
    }
    int_attribs.push([0, 0]);

    let i_ptr = int_attribs.as_ptr();
    let f_ptr = std::ptr::null_mut();
    let mut out_format = 0;
    let mut out_format_count = 0;
    let b = (f.unwrap())(
        hdc,
        i_ptr.cast(),
        f_ptr,
        1,
        &mut out_format,
        &mut out_format_count,
    );

    if b.0 != 0 && out_format_count == 1 {
        return out_format;
    } else {
        panic!();
    }
}

unsafe fn wgl_create_context_attribs(
    f: wglCreateContextAttribsARB_t,
    hdc: HDC,
    h_share_context: HGLRC,
    attrib_list: &[[i32; 2]],
) -> HGLRC {
    let i_ptr = attrib_list.as_ptr();
    let hglrc = (f.unwrap())(hdc, h_share_context, i_ptr.cast());
    if hglrc.0 == 0 {
        panic!("conext attribs is null");
    }
    hglrc
}

fn win32_get_time(time: &mut i64) {
    let res = unsafe { QueryPerformanceCounter(time) };
    if !res.is_ok() {
        let error = unsafe { GetLastError() };
        panic!("failed to query performace counter: {:?}", error);
    }
}

fn win32_duration(perf_freq: i64, start: i64, end: i64) -> f32 {
    (end - start) as f32 / perf_freq as f32
}

fn gl_error(id: &str, wnd_data: *mut WindowData) {
    let e = unsafe { ((*wnd_data).gl.gl_get_error.unwrap())() };
    if e != 0 {
        let mut sources: Vec<GLenum> = Vec::with_capacity(10);
        let mut types: Vec<GLenum> = Vec::with_capacity(10);
        let mut ids: Vec<GLuint> = Vec::with_capacity(10);
        let mut severities: Vec<GLenum> = Vec::with_capacity(10);
        let mut lengths: Vec<GLsizei> = Vec::with_capacity(10);
        let mut message_log: Vec<GLchar> = Vec::with_capacity(500);

        unsafe {
            let res = ((*wnd_data).gl.gl_get_debug_message_log.unwrap())(
                10,
                500,
                sources.as_mut_ptr(),
                types.as_mut_ptr(),
                ids.as_mut_ptr(),
                severities.as_mut_ptr(),
                lengths.as_mut_ptr(),
                message_log.as_mut_ptr(),
            );
            if res == 0 {
                // println!("No debug messages");
            }
            panic!(
                "{}: 0x{:x},\tLog: {}",
                id,
                e,
                String::from_utf8(std::mem::transmute(message_log)).unwrap()
            );
        }
    }
}

unsafe fn win32_render_bmap(wnd_data: *mut WindowData, bmp: &mut BitMap) {
    ((*wnd_data).gl.gl_clear.unwrap())(GL_COLOR_BUFFER_BIT);

    gl_error("0", wnd_data);

    ((*wnd_data).gl.gl_tex_image_2d.unwrap())(
        GL_TEXTURE_2D,
        0,
        GL_RGBA as i32,
        bmp.width as i32,
        bmp.height as i32,
        0,
        GL_RGBA,
        GL_UNSIGNED_INT_8_8_8_8,
        bmp.pixels.as_mut_ptr().cast(),
    );

    gl_error("1", wnd_data);

    ((*wnd_data).gl.gl_blit_framebuffer.unwrap())(
        0,
        0,
        bmp.width as i32,
        bmp.height as i32,
        0,
        0,
        (*wnd_data).tex_data.width as i32,
        (*wnd_data).tex_data.height as i32,
        GL_COLOR_BUFFER_BIT,
        GL_NEAREST,
    );

    gl_error("2", wnd_data);

    // println!("BMAP: {}x{}, TEXTURE: {}x{}", bmap_data.width, bmap_data.height,
    //          (*wnd_data).tex_data.width, (*wnd_data).tex_data.height);
}

type UpdateGameAndRender = fn(game: &mut World, input: &mut InputBuffer, back_buf: &mut BitMap);

fn dyn_load_game(path: &String) -> (SystemTime, HMODULE, UpdateGameAndRender) {
    let buf = read_file(path.into());
    assert_eq!(
        (),
        std::fs::write("target/debug/hot_load.dll", &buf).unwrap()
    );

    let game_lib =
        match unsafe { LoadLibraryW(PCWSTR(wide_null("target/debug/hot_load.dll").as_ptr())) } {
            Ok(lib) => lib,
            Err(_) => {
                let error = unsafe { GetLastError() };
                panic!("failed to load game dll: {:?}", error);
            }
        };

    let update_game_and_render: UpdateGameAndRender = unsafe {
        let addr = GetProcAddress(game_lib, PCSTR(c_str!("update_game_and_render").as_ptr()));
        if addr.is_none() {
            let error = GetLastError();
            panic!("failed to load game function: {:?}", error);
        }

        core::mem::transmute(addr.unwrap())
    };

    trace!("Loading DLL: {}", path);

    (SystemTime::now(), game_lib, update_game_and_render)
}

fn read_file(path: String) -> Vec<u8> {
    let mut f = std::fs::File::open(path).unwrap();
    let metadata = f.metadata().unwrap();
    let mut buf = vec![0u8; metadata.len() as usize];
    let bytes = f.read(&mut buf).unwrap();

    assert_eq!(bytes, metadata.len() as usize);

    buf
}

fn dyn_refresh(
    last_refresh: SystemTime,
    game_lib: HMODULE,
    update: &mut UpdateGameAndRender,
    path: &String,
) -> Option<(HMODULE, SystemTime)> {
    let f = std::fs::File::open(path).unwrap();
    let last_modified = f.metadata().unwrap().modified().unwrap();

    if last_refresh > last_modified {
        return None;
    } else {
        info!("## Refreshing DLL");
    }

    let res = unsafe { FreeLibrary(game_lib) };
    if !res.is_ok() {
        let error = unsafe { GetLastError() };
        panic!("failed to unload game dll: {:?}", error);
    }

    let buf = read_file(path.into());
    std::fs::write("target/debug/hot_load.dll", &buf).unwrap();

    let game_lib =
        match unsafe { LoadLibraryW(PCWSTR(wide_null("target/debug/hot_load.dll").as_ptr())) } {
            Ok(lib) => lib,
            Err(_) => {
                let error = unsafe { GetLastError() };
                panic!("failed to load game dll: {:?}", error);
            }
        };

    unsafe {
        let addr = GetProcAddress(game_lib, PCSTR(c_str!("update_game_and_render").as_ptr()));
        if addr.is_none() {
            let error = GetLastError();
            panic!("failed to load game function: {:?}", error);
        }

        *update = core::mem::transmute(addr.unwrap())
    };

    Some((game_lib, SystemTime::now()))
}

// struct XAudio {
//     x: IXAudio2,
//     mastering_voice: IXAudio2MasteringVoice,
//     source_voice: IXAudio2SourceVoice,
// }

fn xaudio_init(
    sample_rate: usize,
    channels: usize,
    bits_per_sample: usize,
    bytes_per_second: usize,
    buf_bytes: usize,
    audio_buf: &mut [u8],
) -> (IXAudio2, IXAudio2MasteringVoice, IXAudio2SourceVoice) {
    if S_OK != unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) } {
        // TODO: error
        panic!();
    }

    unsafe {
        let mut x = None;
        assert_eq!(
            Ok(()),
            XAudio2CreateWithVersionInfo(&mut x, 0, XAUDIO2_DEFAULT_PROCESSOR, NTDDI_MAXVER)
        );
        assert!(x.is_some());
        let xaudio = x.unwrap();

        let mut mv = None;
        assert_eq!(
            Ok(()),
            xaudio.CreateMasteringVoice(
                &mut mv,
                XAUDIO2_DEFAULT_CHANNELS,
                XAUDIO2_DEFAULT_SAMPLERATE,
                0,
                PCWSTR::null(),
                None,
                AUDIO_STREAM_CATEGORY(0)
            )
        );
        assert!(mv.is_some());
        let mastering_voice = mv.unwrap();

        let mut dbg = XAUDIO2_DEBUG_CONFIGURATION::default();
        dbg.TraceMask = XAUDIO2_LOG_ERRORS | XAUDIO2_LOG_WARNINGS | XAUDIO2_LOG_INFO;
        dbg.BreakMask = XAUDIO2_LOG_ERRORS;
        xaudio.SetDebugConfiguration(Some(&dbg), None);

        let mut wfmt = WAVEFORMATEX::default();
        wfmt.wFormatTag = WAVE_FORMAT_PCM as u16;
        wfmt.nChannels = channels as u16;
        wfmt.nSamplesPerSec = sample_rate as u32;
        wfmt.nAvgBytesPerSec = bytes_per_second as u32;
        wfmt.nBlockAlign = (channels * bits_per_sample) as u16 / 8;
        wfmt.wBitsPerSample = 16;
        wfmt.cbSize = 0;

        let mut sv = None;
        assert_eq!(
            Ok(()),
            xaudio.CreateSourceVoice(
                &mut sv,
                &wfmt,
                0,
                XAUDIO2_DEFAULT_FREQ_RATIO,
                &*IXAudio2VoiceCallback::new(&XAudioCallback),
                None,
                None
            )
        );
        assert!(sv.is_some());
        let source_voice = sv.unwrap();

        let mut pbuf = XAUDIO2_BUFFER::default();
        pbuf.AudioBytes = buf_bytes as u32;
        pbuf.pAudioData = audio_buf.as_mut_ptr();
        pbuf.Flags = XAUDIO2_END_OF_STREAM;
        // pbuf.pContext = audio_buf.as_mut_ptr().cast();
        pbuf.LoopCount = XAUDIO2_LOOP_INFINITE;

        assert_eq!(buf_bytes, audio_buf.len());

        assert_eq!(Ok(()), source_voice.SubmitSourceBuffer(&pbuf, None));
        assert_eq!(Ok(()), source_voice.Start(0, XAUDIO2_COMMIT_NOW));

        GetLastError().unwrap();

        // return XAudio {
        //     x: xaudio,
        //     mastering_voice,
        //     source_voice,
        // }

        (xaudio, mastering_voice, source_voice)
    }
}

struct XAudioCallback;

impl IXAudio2VoiceCallback_Impl for XAudioCallback {
    fn OnVoiceProcessingPassStart(&self, bytesrequired: u32) {
        println!("OnVoiceProcessingPassStart: {}", bytesrequired);
    }
    fn OnVoiceProcessingPassEnd(&self) {
        println!("OnVoiceProcessingPassEnd");
    }
    fn OnStreamEnd(&self) {
        println!("OnStreamEnd");
    }
    fn OnBufferStart(&self, pbuffercontext: *mut ::core::ffi::c_void) {
        //println!("OnBufferStart: {}", unsafe { *pbuffercontext.cast::<u8>() });
        println!("OnBufferStart");
    }
    fn OnBufferEnd(&self, pbuffercontext: *mut ::core::ffi::c_void) {
        println!("OnBufferEnd");
    }
    fn OnLoopEnd(&self, pbuffercontext: *mut ::core::ffi::c_void) {
        println!("OnLoopEnd");
    }
    fn OnVoiceError(&self, pbuffercontext: *mut ::core::ffi::c_void, error: HRESULT) {
        println!("OnVoiceError");
    }
}

pub fn win32_main(reload_path: String, world: &mut World) {
    // We have to create a window to set the pixel format, but then
    // we have to create a new window to actually use it?
    trace!("Initializing OpenGL");
    let (
        extensions,
        wgl_choose_pixel_format_arb,
        wgl_create_context_attribs_arb,
        wgl_swap_interval_ext,
    ) = win32_grab_wgl_pointers();

    // Creating the actual window this time
    trace!("Creating Window");
    let class = wide_null("RegolithHillWindowClass");
    let w_name = wide_null("Regolith Hill");
    let h_instance = win32_register_window_class(class.clone(), Some(win32_window_callback));

    let wnd_data: *mut WindowData = Box::leak(Box::new(WindowData::default()));
    let wnd_width = 1920;
    let wnd_height = 1080;
    let hwnd = win32_create_window(
        wnd_width,
        wnd_height,
        class,
        w_name,
        h_instance,
        WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        Some(wnd_data.cast()),
    );
    unsafe { (*wnd_data).hwnd = hwnd };
    let mut input_buf = InputBuffer::default();
    unsafe { (*wnd_data).input_buf = &mut input_buf };

    // initializing the buffer
    let mut client_rect = RECT::default();
    unsafe {
        let res = GetClientRect(hwnd, &mut client_rect);
        if res.is_err() {
            let error = GetLastError();
            panic!("Could not get size of client rect: {:?}", error);
        }
    }
    let width = client_rect.right;
    let height = client_rect.bottom;

    let pixels: Vec<u32> = vec![0x000000ff; (width * height) as usize];
    let mut back_buf = BitMap::new(pixels, width as usize, height as usize);

    trace!(
        "Allocating New Buffer -- Width: {}, Height: {}",
        back_buf.width,
        back_buf.height
    );

    let hdc = unsafe { GetDC(hwnd) };
    unsafe { (*wnd_data).hdc = hdc };

    let pix_format =
        unsafe { wgl_choose_pixel_format(wgl_choose_pixel_format_arb, hdc, &extensions) };
    let mut pfd = PIXELFORMATDESCRIPTOR::default();
    unsafe {
        DescribePixelFormat(
            hdc,
            pix_format,
            std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as _,
            Some(&mut pfd),
        )
    };
    unsafe { SetPixelFormat(hdc, pix_format, &pfd) };

    // creating the opengl context
    trace!("Creating OpenGL Context");
    const FLAGS: c_int = WGL_CONTEXT_FORWARD_COMPATIBLE_BIT_ARB
        | if cfg!(debug_assertions) {
            WGL_CONTEXT_DEBUG_BIT_ARB
        } else {
            0
        };
    let hglrc = unsafe {
        wgl_create_context_attribs(
            wgl_create_context_attribs_arb,
            hdc,
            HGLRC::default(),
            &[
                [WGL_CONTEXT_MAJOR_VERSION_ARB, 3],
                [WGL_CONTEXT_MINOR_VERSION_ARB, 3],
                [
                    WGL_CONTEXT_PROFILE_MASK_ARB,
                    WGL_CONTEXT_CORE_PROFILE_BIT_ARB,
                ],
                [WGL_CONTEXT_FLAGS_ARB, FLAGS],
                [0, 0],
            ],
        )
    };
    unsafe { wglMakeCurrent(hdc, hglrc) };
    unsafe { (*wnd_data).hglrc = hglrc };

    let opengl32 = unsafe { LoadLibraryW(PCWSTR(wide_null("opengl32.dll").as_ptr())) };
    if opengl32.is_err() {
        let error = unsafe { GetLastError() };
        panic!("failed to load opengl32 library: {:?}", error);
    }
    let gl = unsafe { GL::bind_functions(opengl32.unwrap(), wnd_data) };
    unsafe { (*wnd_data).gl = gl };

    gl_error("init", wnd_data);

    // vsync
    if extensions.iter().any(|s| s == "WGL_EXT_swap_control_tear") {
        unsafe { (wgl_swap_interval_ext.unwrap())(-1) };
    } else {
        unsafe { (wgl_swap_interval_ext.unwrap())(1) };
    }

    // audio stuffs
    // trace!("Initializing Audio");
    // let sample_rate = 44100;
    // let channels = 2;
    // let bits_per_sample = 16;
    // let bytes_per_second = sample_rate * channels * bits_per_sample / 8;
    // let buf_bytes = 2 * bytes_per_second;
    // let mut audio_buf = vec![0u8; buf_bytes];

    // let freq = 220.0;

    // for (i, b) in audio_buf.iter_mut().enumerate() {
    //     *b = (i as f32 * 2.0 * 3.145 * freq / sample_rate as f32).cos() as u8;
    // }

    // let xaudio =
    // let (xaudio, mastering_voice, source_voice) = xaudio_init(
    //     sample_rate,
    //     channels,
    //     bits_per_sample,
    //     bytes_per_second,
    //     buf_bytes,
    //     &mut audio_buf,
    // );
    // println!("{:?}", audio_buf.len());

    // frame rate things
    let target_frame_rate = 60.0;
    let target_frame_len = 1.0 / (target_frame_rate);
    let mut perf_freq = 0;
    unsafe { QueryPerformanceFrequency(&mut perf_freq) };

    // performace things
    let mut frames = 0;
    let mut total_frames = 0;
    let mut lost_frames = 0;
    let mut lost_frames_sum = 0;
    let mut highest_lost_frames = 0;
    let mut frames_sum = 0.0;
    let mut iterations = 0;
    let mut fps_test = 0;
    let mut end_fps_test = 0;
    win32_get_time(&mut fps_test);

    // making the texture and framebuffer to draw the bitmap to
    let tex_data = TextureData {
        width: back_buf.width,
        height: back_buf.height,
    };
    unsafe { (*wnd_data).tex_data = tex_data };

    let mut tex_id: GLuint = 0;
    let mut fb_id: GLuint = 0;
    unsafe {
        ((*wnd_data).gl.gl_gen_textures.unwrap())(1, &mut tex_id);
        ((*wnd_data).gl.gl_bind_texture.unwrap())(GL_TEXTURE_2D, tex_id);

        ((*wnd_data).gl.gl_tex_parameteri.unwrap())(
            GL_TEXTURE_2D,
            GL_TEXTURE_MIN_FILTER,
            GL_NEAREST as i32,
        );
        ((*wnd_data).gl.gl_tex_parameteri.unwrap())(
            GL_TEXTURE_2D,
            GL_TEXTURE_MAG_FILTER,
            GL_NEAREST as i32,
        );

        ((*wnd_data).gl.gl_gen_framebuffers.unwrap())(1, &mut fb_id);
        ((*wnd_data).gl.gl_bind_framebuffer.unwrap())(GL_READ_FRAMEBUFFER, fb_id);
        ((*wnd_data).gl.gl_framebuffer_texture_2d.unwrap())(
            GL_READ_FRAMEBUFFER,
            GL_COLOR_ATTACHMENT0,
            GL_TEXTURE_2D,
            tex_id,
            0,
        );
        ((*wnd_data).gl.gl_bind_framebuffer.unwrap())(GL_DRAW_FRAMEBUFFER, 0);
    }
    gl_error("textures", wnd_data);

    let show_perf = false;

    // NOTE: for the purposes of hot reloading, these are the function pointers
    // that reference regolith_hill.dll
    let (mut last_refresh, mut game_lib, mut update_game_and_render) = dyn_load_game(&reload_path);

    // game loop
    let mut game_loop = true;
    trace!("Entering Game Loop");
    while game_loop {
        // unsafe {
        //     let mut pvoicestate = XAUDIO2_VOICE_STATE::default();
        //     let v = xaudio.source_voice.GetVolume();
        //     xaudio.source_voice.GetState(&mut pvoicestate, 0);
        //     let smp = pvoicestate.SamplesPlayed;
        //     println!("STATE: {}, {}, {}", v, smp, (pvoicestate.BuffersQueued > 0));
        // }

        let mut start_count = 0;
        win32_get_time(&mut start_count);

        // messages
        let mut msg = MSG::default();
        if unsafe { PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE) }.0 != 0 {
            match msg.message {
                WM_QUIT => game_loop = false,
                _ => unsafe {
                    TranslateMessage(&mut msg);
                    DispatchMessageW(&mut msg);
                },
            }

            (update_game_and_render)(world, &mut input_buf, &mut back_buf);

            // rendering

            // First, we have to create a texture for the bitmap supplied by the game layer.
            // Next, we pass that into the render call which loads that texture into a
            // fram buffer.
            unsafe {
                win32_render_bmap(wnd_data, &mut back_buf);

                let mut work_count = 0;
                win32_get_time(&mut work_count);
                let mut current_frame_len = win32_duration(perf_freq, start_count, work_count);

                let unlimited_fps = false;

                if !unlimited_fps {
                    if current_frame_len < target_frame_len {
                        while current_frame_len < target_frame_len {
                            let mut wait_count = 0;
                            win32_get_time(&mut wait_count);
                            current_frame_len = win32_duration(perf_freq, start_count, wait_count);
                        }
                    } else {
                        lost_frames += 1;
                        lost_frames_sum += 1;
                    }
                }

                if show_perf {
                    trace!(
                        "> Measured Frame Length: {},\tTarget Frame Length: {},\tLoss: {}",
                        current_frame_len,
                        target_frame_len,
                        (current_frame_len - target_frame_len).abs()
                    );
                }
                frames_sum += current_frame_len;

                SwapBuffers((*wnd_data).hdc);
            }

            match dyn_refresh(
                last_refresh,
                game_lib,
                &mut update_game_and_render,
                &reload_path,
            ) {
                Some((dll, time)) => {
                    game_lib = dll;
                    last_refresh = time
                }
                None => {}
            }

            win32_get_time(&mut end_fps_test);
            let duration = unsafe { win32_duration(perf_freq, fps_test, end_fps_test) };

            frames += 1;
            if duration >= 1.0 {
                total_frames += frames;

                if show_perf {
                    trace!(
                        "< Frames {},\tDuration: {},\tExpected {} Frames: {},\tLost Frames: {}",
                        frames,
                        duration,
                        frames,
                        frames_sum,
                        lost_frames
                    );
                }

                win32_get_time(&mut fps_test);
                if lost_frames > highest_lost_frames {
                    highest_lost_frames = lost_frames;
                }
                frames = 0;
                lost_frames = 0;
                frames_sum = 0.0;
                iterations += 1;
            }
        }
    }
    info!(
        ">> Iterations: {},\tFPS: {},\tTotal Lost Frames: {},\tAverage: {},\tHigh:{}",
        iterations,
        total_frames / iterations,
        lost_frames_sum,
        lost_frames_sum / iterations,
        highest_lost_frames
    );
}
