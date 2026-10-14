use gtk::prelude::*;
use std::cell::RefCell;
use std::env;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use webkit2gtk::{SettingsExt, WebView, WebViewExt};

const BACKEND_URL: &str = "http://127.0.0.1/local";
const BOOT_HTML: &str = "file:///opt/interface/static/htmls/boot.html";
const ERROR_HTML: &str = "file:///opt/interface/static/htmls/error.html";
const BOOT_TIMEOUT_SECS: u64 = 10;
const HEALTH_POLL_INTERVAL_SECS: u64 = 2;
const HTTP_REQ_TIMEOUT_MS: u64 = 1500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisplayState {
    Boot,
    App,
    Error,
}

fn main() {
    // Force WebKit hardware acceleration and disable AT-SPI D-Bus lookups on headless/bare X11
    unsafe {
        env::set_var("WEBKIT_FORCE_COMPOSITING_MODE", "1");
        env::set_var("NO_AT_BRIDGE", "1");
    }

    if gtk::init().is_err() {
        eprintln!("[fatal] Failed to initialize GTK context. Check DISPLAY variable.");
        return;
    }

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("NVR Kiosk Overlay");
    window.set_decorated(false);

    // Query physical screen boundaries to prevent fractional viewports under bare X11/xinit
    if let Some(screen) = gdk::Screen::default() {
        let width = screen.width();
        let height = screen.height();
        window.set_default_size(width, height);
        window.resize(width, height);
        window.move_(0, 0);
    }

    let webview = WebView::new();
    if let Some(settings) = WebViewExt::settings(&webview) {
        settings.set_enable_webgl(true);
        settings.set_enable_javascript(true);
        settings.set_enable_page_cache(false);
    }
    webview.set_hexpand(true);
    webview.set_vexpand(true);
    window.add(&webview);

    window.maximize();
    window.fullscreen();
    window.connect_destroy(|_| gtk::main_quit());

    let active_state = Rc::new(RefCell::new(DisplayState::Boot));
    webview.load_uri(BOOT_HTML);
    window.show_all();

    let (state_sender, state_receiver) =
        glib::MainContext::channel::<DisplayState>(glib::PRIORITY_DEFAULT);

    // Handle UI updates within the GTK main loop thread
    let state_ref = Rc::clone(&active_state);
    let webview_ref = webview.clone();
    state_receiver.attach(None, move |next_state| {
        if *state_ref.borrow() != next_state {
            *state_ref.borrow_mut() = next_state;
            match next_state {
                DisplayState::App => webview_ref.load_uri(BACKEND_URL),
                DisplayState::Error => webview_ref.load_uri(ERROR_HTML),
                DisplayState::Boot => webview_ref.load_uri(BOOT_HTML),
            }
        }
        glib::Continue(true)
    });

    let running = Arc::new(AtomicBool::new(true));
    let is_running = Arc::clone(&running);

    // Background watcher thread for backend readiness & continuous health monitoring
    thread::spawn(move || {
        let boot_start = Instant::now();
        let mut backend_available = false;

        while boot_start.elapsed() < Duration::from_secs(BOOT_TIMEOUT_SECS) {
            if is_backend_alive() {
                backend_available = true;
                let _ = state_sender.send(DisplayState::App);
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }

        if !backend_available && !is_backend_alive() {
            let _ = state_sender.send(DisplayState::Error);
        }

        let mut connection_established = backend_available;

        while is_running.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_secs(HEALTH_POLL_INTERVAL_SECS));
            let alive = is_backend_alive();

            if alive && !connection_established {
                connection_established = true;
                let _ = state_sender.send(DisplayState::App);
            } else if !alive && connection_established {
                connection_established = false;
                let _ = state_sender.send(DisplayState::Error);
            }
        }
    });

    gtk::main();
    running.store(false, Ordering::Relaxed);
}

fn is_backend_alive() -> bool {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(HTTP_REQ_TIMEOUT_MS))
        .timeout_read(Duration::from_millis(HTTP_REQ_TIMEOUT_MS))
        .build();

    match agent
        .get(BACKEND_URL)
        .set("User-Agent", "NVR-Overlay/1.0")
        .call()
    {
        Ok(res) => res.status() == 200 || res.status() == 304,
        Err(_) => false,
    }
}
