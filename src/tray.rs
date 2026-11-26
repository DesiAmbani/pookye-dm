//! The notification-area icon, with a real Win32 message window.
//!
//! The window is frameless, so closing it is a request the app can answer.
//! When "close to the tray" is on, the window is hidden instead of destroyed:
//! the download manager, its worker threads and the loopback browser bridge
//! keep running, and the tray icon brings the window back or quits for real.
//!
//! The tray lives on its own thread with its own message pump, so it does not
//! depend on eframe's event loop. The app polls [`poll`] once per frame.

use std::sync::atomic::{AtomicBool, Ordering};

static SHOW: AtomicBool = AtomicBool::new(false);
static QUIT: AtomicBool = AtomicBool::new(false);

/// What the person using the tray icon asked for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrayEvent {
    /// Open the window again.
    Show,
    /// Leave for good.
    Quit,
}

/// Takes the next request made through the tray icon, if there is one.
pub fn poll() -> Option<TrayEvent> {
    if QUIT.swap(false, Ordering::Relaxed) {
        return Some(TrayEvent::Quit);
    }
    if SHOW.swap(false, Ordering::Relaxed) {
        return Some(TrayEvent::Show);
    }
    None
}

#[cfg(windows)]
pub use win::{AppWindow, Tray};

#[cfg(not(windows))]
pub struct Tray;

#[cfg(not(windows))]
impl Tray {
    pub fn new() -> Option<Self> {
        None
    }
}

/// A handle to the window eframe created, for parking it off-screen.
#[cfg(not(windows))]
pub struct AppWindow;

#[cfg(not(windows))]
impl AppWindow {
    pub fn find() -> Option<Self> {
        None
    }

    pub fn strip_frame(&self) {}

    pub fn alive(&self) -> bool {
        false
    }

    pub fn work_area(&self) -> Option<(i32, i32, i32, i32)> {
        None
    }

    pub fn cursor(&self) -> Option<(i32, i32)> {
        None
    }

    pub fn origin(&self) -> Option<(i32, i32)> {
        None
    }

    pub fn park(&self) -> Option<(i32, i32)> {
        None
    }

    pub fn restore(&self, _at: Option<(i32, i32)>) {}
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    use std::ptr;
    use std::sync::atomic::Ordering;
    use std::sync::mpsc::{channel, Sender};
    use std::thread::JoinHandle;
    use std::time::Duration;

    type Handle = isize;
    type WndProc = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;

    const WM_DESTROY: u32 = 0x0002;
    const WM_CLOSE: u32 = 0x0010;
    const WM_NULL: u32 = 0x0000;
    const WM_LBUTTONUP: u32 = 0x0202;
    const WM_LBUTTONDBLCLK: u32 = 0x0203;
    const WM_RBUTTONUP: u32 = 0x0205;
    const WM_APP_TRAY: u32 = 0x8000 + 1;
    const WM_COMMAND_OPEN: usize = 1;
    const WM_COMMAND_QUIT: usize = 2;

    const NIM_ADD: u32 = 0;
    const NIM_DELETE: u32 = 2;
    const NIF_MESSAGE: u32 = 0x01;
    const NIF_ICON: u32 = 0x02;
    const NIF_TIP: u32 = 0x04;
    const IMAGE_ICON: u32 = 1;
    const LR_LOADFROMFILE: u32 = 0x0010;
    const MF_STRING: u32 = 0x0000;
    const MF_SEPARATOR: u32 = 0x0800;
    const TPM_RIGHTBUTTON: u32 = 0x0002;
    const TPM_RETURNCMD: u32 = 0x0100;
    const SM_CXSMICON: i32 = 49;
    const IDI_APPLICATION: usize = 32512;
    #[repr(C)]
    #[derive(Default)]
    struct MonitorInfo {
        size: u32,
        monitor: WindowRect,
        work: WindowRect,
        flags: u32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn MonitorFromWindow(hwnd: Handle, flags: u32) -> Handle;
        fn GetMonitorInfoW(monitor: Handle, info: *mut MonitorInfo) -> i32;
    }

    const MONITOR_DEFAULTTONEAREST: u32 = 2;
    const GWL_STYLE: i32 = -16;
    const GWL_EXSTYLE: i32 = -20;
    const WS_CAPTION: i32 = 0x00C0_0000;
    const WS_THICKFRAME: i32 = 0x0004_0000;
    const SWP_FRAMECHANGED: u32 = 0x0020;
    const WS_EX_TOOLWINDOW: i32 = 0x0000_0080;
    const WS_EX_APPWINDOW: i32 = 0x0004_0000;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SW_SHOW: i32 = 5;


    #[repr(C)]
    struct WndClassW {
        style: u32,
        wnd_proc: Option<WndProc>,
        class_extra: i32,
        window_extra: i32,
        instance: Handle,
        icon: Handle,
        cursor: Handle,
        background: Handle,
        menu_name: *const u16,
        class_name: *const u16,
    }

    #[repr(C)]
    #[derive(Default)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct WindowRect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[repr(C)]
    struct Msg {
        hwnd: Handle,
        message: u32,
        wparam: usize,
        lparam: isize,
        time: u32,
        point: Point,
    }

    /// NOTIFYICONDATAW as laid out by `winuser.h`, Vista and later sizes.
    #[repr(C)]
    struct NotifyIconDataW {
        size: u32,
        hwnd: Handle,
        id: u32,
        flags: u32,
        callback_message: u32,
        icon: Handle,
        tip: [u16; 128],
        state: u32,
        state_mask: u32,
        info: [u16; 256],
        timeout_or_version: u32,
        info_title: [u16; 64],
        info_flags: u32,
        guid: Guid,
        balloon_icon: Handle,
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> Handle;
        fn RegisterClassW(class: *const WndClassW) -> u16;
        fn CreateWindowExW(
            ex_style: u32,
            class: *const u16,
            name: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Handle,
            menu: Handle,
            instance: Handle,
            param: *mut c_void,
        ) -> Handle;
        fn DefWindowProcW(hwnd: Handle, msg: u32, wparam: usize, lparam: isize) -> isize;
        fn DestroyWindow(hwnd: Handle) -> i32;
        fn PostQuitMessage(code: i32);
        fn GetMessageW(msg: *mut Msg, hwnd: Handle, min: u32, max: u32) -> i32;
        fn TranslateMessage(msg: *const Msg) -> i32;
        fn DispatchMessageW(msg: *const Msg) -> isize;
        fn PostMessageW(hwnd: Handle, msg: u32, wparam: usize, lparam: isize) -> i32;
        fn LoadImageW(
            instance: Handle,
            name: *const u16,
            kind: u32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> Handle;
        fn LoadIconW(instance: Handle, name: *const u16) -> Handle;
        fn DestroyIcon(icon: Handle) -> i32;
        fn CreatePopupMenu() -> Handle;
        fn AppendMenuW(menu: Handle, flags: u32, id: usize, text: *const u16) -> i32;
        fn DestroyMenu(menu: Handle) -> i32;
        fn TrackPopupMenu(
            menu: Handle,
            flags: u32,
            x: i32,
            y: i32,
            reserved: i32,
            hwnd: Handle,
            rect: *const c_void,
        ) -> i32;
        fn GetCursorPos(point: *mut Point) -> i32;
        fn SetForegroundWindow(hwnd: Handle) -> i32;
        fn GetSystemMetrics(index: i32) -> i32;
        fn FindWindowW(class: *const u16, name: *const u16) -> Handle;
        fn IsWindow(hwnd: Handle) -> i32;
        fn GetWindowRect(hwnd: Handle, rect: *mut WindowRect) -> i32;
        fn GetWindowLongW(hwnd: Handle, index: i32) -> i32;
        fn SetWindowLongW(hwnd: Handle, index: i32, value: i32) -> i32;
        fn SetWindowPos(
            hwnd: Handle,
            after: Handle,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
        fn ShowWindow(hwnd: Handle, command: i32) -> i32;
    }

    #[link(name = "shell32")]
    extern "system" {
        fn Shell_NotifyIconW(op: u32, data: *mut NotifyIconDataW) -> i32;
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// The app's own window, so closing can park it out of sight instead of
    /// hiding it: a hidden winit window stops receiving redraws, and with them
    /// the frame that pumps the queue and the browser bridge.
    pub struct AppWindow {
        hwnd: Handle,
    }

    impl AppWindow {
        pub fn find() -> Option<Self> {
            let title = wide("Pookye Download Manager");
            let hwnd = unsafe { FindWindowW(ptr::null(), title.as_ptr()) };
            if hwnd == 0 {
                return None;
            }
            Some(Self { hwnd })
        }

        /// The mouse position on screen, which does not move when the window does.
        pub fn cursor(&self) -> Option<(i32, i32)> {
            let mut point = Point::default();
            if unsafe { GetCursorPos(&mut point) } == 0 {
                return None;
            }
            Some((point.x, point.y))
        }

        /// The usable area of the monitor the window is on: x, y, width, height.
        /// Maximizing to this by hand avoids the framed client Windows uses for
        /// a native maximize, which left a dead band at the bottom.
        pub fn work_area(&self) -> Option<(i32, i32, i32, i32)> {
            let monitor = unsafe { MonitorFromWindow(self.hwnd, MONITOR_DEFAULTTONEAREST) };
            if monitor == 0 {
                return None;
            }
            let mut info = MonitorInfo {
                ..MonitorInfo::default()
            };
            info.size = std::mem::size_of::<MonitorInfo>() as u32;
            if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
                return None;
            }
            let rect = info.work;
            Some((
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
            ))
        }

        /// The window's position on screen.
        pub fn origin(&self) -> Option<(i32, i32)> {
            let mut rect = WindowRect::default();
            if unsafe { GetWindowRect(self.hwnd, &mut rect) } == 0 {
                return None;
            }
            Some((rect.left, rect.top))
        }

        /// Whether the handle still belongs to a live window. A handle kept from
        /// start-up goes stale if the window is recreated, and a stale one answers
        /// every style call with zero.
        pub fn alive(&self) -> bool {
            unsafe { IsWindow(self.hwnd) != 0 }
        }

        /// Drops the caption and resize frame styles that winit keeps even for a
        /// frameless window. Windows lays out the drawn surface inside that frame
        /// otherwise, which shifts the whole interface down and right.
        pub fn strip_frame(&self) {
            unsafe {
                let style = GetWindowLongW(self.hwnd, GWL_STYLE);
                let cleaned = style & !(WS_CAPTION | WS_THICKFRAME);
                if cleaned == style {
                    return;
                }
                SetWindowLongW(self.hwnd, GWL_STYLE, cleaned);
                SetWindowPos(
                    self.hwnd,
                    0,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }
        }

        /// Puts the window in front of the normal z-order and asks for the
        /// keyboard. Windows only raises a window itself when it activates it,
        /// and it can leave a borderless window behind a covering one; the
        /// z-order raise is a call of its own so the window at least appears
        /// where the click says it should.
        pub fn raise(&self) {
            unsafe {
                SetWindowPos(
                    self.hwnd,
                    0,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE,
                );
                SetForegroundWindow(self.hwnd);
            }
        }

        /// Moves the window off-screen, drops its taskbar button and returns
        /// where it was, so it can be put back exactly there.
        pub fn park(&self) -> Option<(i32, i32)> {
            unsafe {
                let mut rect = WindowRect::default();
                GetWindowRect(self.hwnd, &mut rect);
                let style = GetWindowLongW(self.hwnd, GWL_EXSTYLE);
                SetWindowLongW(
                    self.hwnd,
                    GWL_EXSTYLE,
                    (style | WS_EX_TOOLWINDOW) & !WS_EX_APPWINDOW,
                );
                SetWindowPos(
                    self.hwnd,
                    0,
                    -32000,
                    -32000,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
                Some((rect.left, rect.top))
            }
        }

        pub fn restore(&self, at: Option<(i32, i32)>) {
            unsafe {
                let style = GetWindowLongW(self.hwnd, GWL_EXSTYLE);
                SetWindowLongW(
                    self.hwnd,
                    GWL_EXSTYLE,
                    (style & !WS_EX_TOOLWINDOW) | WS_EX_APPWINDOW,
                );
                let (x, y) = at.unwrap_or((120, 120));
                SetWindowPos(
                    self.hwnd,
                    0,
                    x,
                    y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
                ShowWindow(self.hwnd, SW_SHOW);
                self.raise();
            }
        }
    }

    pub struct Tray {
        hwnd: Handle,
        thread: Option<JoinHandle<()>>,
    }

    impl Tray {
        /// Starts the tray thread and waits for its hidden window to exist.
        pub fn new() -> Option<Self> {
            let (tx, rx) = channel();
            let thread = std::thread::Builder::new()
                .name("pookye-tray".to_string())
                .spawn(move || unsafe { run(tx) })
                .ok()?;
            let hwnd = rx.recv_timeout(Duration::from_secs(5)).ok()?;
            if hwnd == 0 {
                let _ = thread.join();
                return None;
            }
            Some(Self {
                hwnd,
                thread: Some(thread),
            })
        }
    }

    impl Drop for Tray {
        fn drop(&mut self) {
            unsafe {
                PostMessageW(self.hwnd, WM_CLOSE, 0, 0);
            }
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    unsafe fn run(tx: Sender<Handle>) {
        let instance = GetModuleHandleW(ptr::null());
        let class_name = wide("PookyeTraySink");
        let class = WndClassW {
            style: 0,
            wnd_proc: Some(wndproc),
            class_extra: 0,
            window_extra: 0,
            instance,
            icon: 0,
            cursor: 0,
            background: 0,
            menu_name: ptr::null(),
            class_name: class_name.as_ptr(),
        };
        if RegisterClassW(&class) == 0 {
            eprintln!("pookye tray: the window class could not be registered");
            let _ = tx.send(0);
            return;
        }
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            instance,
            ptr::null_mut(),
        );
        if hwnd == 0 {
            eprintln!("pookye tray: the hidden window could not be created");
            let _ = tx.send(0);
            return;
        }

        let icon = load_icon();
        let mut data = NotifyIconDataW {
            size: std::mem::size_of::<NotifyIconDataW>() as u32,
            hwnd,
            id: 1,
            flags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            callback_message: WM_APP_TRAY,
            icon,
            tip: [0; 128],
            state: 0,
            state_mask: 0,
            info: [0; 256],
            timeout_or_version: 0,
            info_title: [0; 64],
            info_flags: 0,
            guid: Guid::default(),
            balloon_icon: 0,
        };
        let tip = wide("Pookye DM — downloads keep running");
        for (slot, value) in data.tip.iter_mut().zip(tip) {
            *slot = value;
        }
        let added = Shell_NotifyIconW(NIM_ADD, &mut data) != 0;
        if !added {
            eprintln!("pookye tray: Windows refused the tray icon");
        } else {
            eprintln!("pookye tray: icon added");
        }
        let _ = tx.send(hwnd);

        let mut msg = std::mem::zeroed::<Msg>();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        Shell_NotifyIconW(NIM_DELETE, &mut data);
        if icon != 0 {
            DestroyIcon(icon);
        }
    }

    /// The brand mark, from an ICO written next to the temporary folder.
    unsafe fn load_icon() -> Handle {
        let size = GetSystemMetrics(SM_CXSMICON).max(16);
        let path = std::env::temp_dir().join(format!("pookye-tray-{}.ico", std::process::id()));
        if std::fs::write(&path, include_bytes!("../assets/pookye.ico")).is_ok() {
            let name = wide(&path.to_string_lossy());
            let icon = LoadImageW(0, name.as_ptr(), IMAGE_ICON, size, size, LR_LOADFROMFILE);
            let _ = std::fs::remove_file(&path);
            if icon != 0 {
                return icon;
            }
        }
        LoadIconW(0, IDI_APPLICATION as *const u16)
    }

    unsafe fn show_menu(hwnd: Handle) {
        let menu = CreatePopupMenu();
        if menu == 0 {
            return;
        }
        let open = wide("Open Pookye DM");
        let quit = wide("Quit Pookye DM");
        AppendMenuW(menu, MF_STRING, WM_COMMAND_OPEN, open.as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
        AppendMenuW(menu, MF_STRING, WM_COMMAND_QUIT, quit.as_ptr());
        let mut point = Point::default();
        GetCursorPos(&mut point);
        SetForegroundWindow(hwnd);
        let command = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_RETURNCMD,
            point.x,
            point.y,
            0,
            hwnd,
            ptr::null(),
        );
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(menu);
        match command as usize {
            WM_COMMAND_OPEN => super::SHOW.store(true, Ordering::Relaxed),
            WM_COMMAND_QUIT => super::QUIT.store(true, Ordering::Relaxed),
            _ => {}
        }
    }

    unsafe extern "system" fn wndproc(hwnd: Handle, msg: u32, wparam: usize, lparam: isize) -> isize {
        match msg {
            WM_APP_TRAY => {
                match (lparam as u32) & 0xFFFF {
                    WM_LBUTTONUP | WM_LBUTTONDBLCLK => super::SHOW.store(true, Ordering::Relaxed),
                    WM_RBUTTONUP => show_menu(hwnd),
                    _ => {}
                }
                0
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
