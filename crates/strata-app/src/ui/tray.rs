//! A notification-area icon owns a dedicated Windows message loop.
//! Failure to create the icon leaves normal window-close behavior intact.
#[derive(Clone, Copy)]
pub enum Event {
    Open,
    Exit,
}

#[cfg(not(windows))]
pub struct Tray;
#[cfg(not(windows))]
impl Tray {
    pub fn new(_: eframe::egui::Context) -> Option<Self> {
        None
    }
    pub fn event(&self) -> Option<Event> {
        None
    }
    pub fn refresh_language(&self) {}
}

#[cfg(windows)]
pub use windows::Tray;

#[cfg(windows)]
mod windows {
    use super::Event;
    use std::{
        ptr::{null, null_mut},
        sync::mpsc,
    };
    use windows_sys::Win32::{
        Foundation::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{Shell::*, WindowsAndMessaging::*},
    };

    const CALLBACK: u32 = WM_APP + 40;
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }
    struct State {
        sender: mpsc::Sender<Event>,
        ctx: eframe::egui::Context,
        restart: u32,
    }
    pub struct Tray {
        hwnd: usize,
        events: mpsc::Receiver<Event>,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    unsafe fn icon(hwnd: HWND) -> NOTIFYICONDATAW {
        let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = hwnd;
        data.uID = 1;
        data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        data.uCallbackMessage = CALLBACK;
        data.hIcon = unsafe { LoadIconW(GetModuleHandleW(null()), 101usize as *const u16) };
        for (slot, value) in data
            .szTip
            .iter_mut()
            .zip(wide("StrataFetch — downloads running in background"))
        {
            *slot = value;
        }
        data
    }

    unsafe extern "system" fn window(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        if message == WM_NCCREATE {
            let create = unsafe { &*(l as *const CREATESTRUCTW) };
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
            }
        }
        let state = unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State).as_ref() };
        if let Some(state) = state {
            if message == state.restart {
                unsafe {
                    Shell_NotifyIconW(NIM_ADD, &icon(hwnd));
                }
                return 0;
            }
            if message == CALLBACK {
                let event = match l as u32 {
                    WM_LBUTTONUP | WM_LBUTTONDBLCLK => Some(Event::Open),
                    WM_RBUTTONUP | WM_CONTEXTMENU => {
                        let menu = unsafe { CreatePopupMenu() };
                        if menu.is_null() {
                            return 0;
                        }
                        unsafe {
                            AppendMenuW(
                                menu,
                                MF_STRING,
                                1,
                                wide(strata_app::i18n::tr("Open StrataFetch")).as_ptr(),
                            );
                            AppendMenuW(
                                menu,
                                MF_STRING,
                                2,
                                wide(strata_app::i18n::tr("Exit StrataFetch")).as_ptr(),
                            );
                        }
                        let mut point: POINT = unsafe { std::mem::zeroed() };
                        unsafe {
                            GetCursorPos(&mut point);
                            SetForegroundWindow(hwnd);
                        }
                        let selected = unsafe {
                            TrackPopupMenu(
                                menu,
                                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                                point.x,
                                point.y,
                                0,
                                hwnd,
                                null(),
                            )
                        };
                        unsafe {
                            DestroyMenu(menu);
                            PostMessageW(hwnd, WM_NULL, 0, 0);
                        }
                        match selected {
                            1 => Some(Event::Open),
                            2 => Some(Event::Exit),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                if let Some(event) = event {
                    let _ = state.sender.send(event);
                    state.ctx.request_repaint();
                }
                return 0;
            }
        }
        match message {
            WM_CLOSE => {
                unsafe {
                    DestroyWindow(hwnd);
                }
                0
            }
            WM_DESTROY => {
                unsafe {
                    Shell_NotifyIconW(NIM_DELETE, &icon(hwnd));
                    PostQuitMessage(0);
                }
                0
            }
            _ => unsafe { DefWindowProcW(hwnd, message, w, l) },
        }
    }

    impl Tray {
        pub fn new(ctx: eframe::egui::Context) -> Option<Self> {
            let (sender, events) = mpsc::channel();
            let (ready, receiver) = mpsc::sync_channel(1);
            let thread = std::thread::spawn(move || unsafe {
                let class = wide("StrataFetch.NotificationArea");
                let instance = GetModuleHandleW(null());
                let mut wc: WNDCLASSW = std::mem::zeroed();
                wc.lpfnWndProc = Some(window);
                wc.hInstance = instance;
                wc.lpszClassName = class.as_ptr();
                if RegisterClassW(&wc) == 0 {
                    return;
                }
                let state = Box::new(State {
                    sender,
                    ctx,
                    restart: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
                });
                // A hidden top-level window receives Explorer's restart broadcast.
                let hwnd = CreateWindowExW(
                    0,
                    class.as_ptr(),
                    wide("StrataFetch tray").as_ptr(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    null_mut(),
                    null_mut(),
                    instance,
                    (&*state as *const State).cast(),
                );
                if hwnd.is_null() {
                    UnregisterClassW(class.as_ptr(), instance);
                    return;
                }
                let data = icon(hwnd);
                if data.hIcon.is_null() || Shell_NotifyIconW(NIM_ADD, &data) == 0 {
                    DestroyWindow(hwnd);
                    UnregisterClassW(class.as_ptr(), instance);
                    return;
                }
                if ready.send(hwnd as usize).is_err() {
                    DestroyWindow(hwnd);
                }
                let mut message: MSG = std::mem::zeroed();
                while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                UnregisterClassW(class.as_ptr(), instance);
                drop(state);
            });
            match receiver.recv() {
                Ok(hwnd) => Some(Self {
                    hwnd,
                    events,
                    thread: Some(thread),
                }),
                Err(_) => {
                    let _ = thread.join();
                    None
                }
            }
        }
        pub fn event(&self) -> Option<Event> {
            self.events.try_recv().ok()
        }
        pub fn refresh_language(&self) {
            unsafe {
                Shell_NotifyIconW(NIM_MODIFY, &icon(self.hwnd as HWND));
            }
        }
    }
    impl Drop for Tray {
        fn drop(&mut self) {
            unsafe {
                PostMessageW(self.hwnd as HWND, WM_CLOSE, 0, 0);
            }
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}
