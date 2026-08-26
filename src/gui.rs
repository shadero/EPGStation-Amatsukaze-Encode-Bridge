use std::{
    io::{self, Write},
    sync::Mutex,
};

use anyhow::{Context, Result};
use iced::{
    futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender},
    widget::{container, scrollable, text},
    window, Element, Font, Length, Subscription, Task, Theme,
};
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use windows_sys::Win32::{
    System::Console::{GetConsoleProcessList, GetConsoleWindow},
    UI::WindowsAndMessaging::{ShowWindow, SW_HIDE},
};

use crate::{config::Config, run_server};

const APP_NAME: &str = "EPGStation–Amatsukaze Encode Bridge";
const MAX_LOG_BYTES: usize = 200_000;

pub(crate) type LogReceiver = UnboundedReceiver<String>;

pub(crate) fn init_logging() -> LogReceiver {
    let (sender, receiver) = mpsc::unbounded();
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,fontdb=error")),
        )
        .with_writer(move || LogWriter(sender.clone()))
        .init();
    receiver
}

pub(crate) fn run(config: Config, logs: LogReceiver) -> Result<()> {
    hide_owned_console();

    let (shutdown, mut shutdown_rx) = watch::channel(false);
    std::thread::spawn(move || {
        let result = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("failed to start server runtime")
            .and_then(|runtime| {
                runtime.block_on(run_server(config, async move {
                    tokio::select! {
                        _ = shutdown_rx.changed() => {}
                        _ = tokio::signal::ctrl_c() => {}
                    }
                }))
            });
        if let Err(error) = result {
            tracing::error!(error = %format!("{error:#}"), "bridge stopped");
        }
    });

    let startup = Mutex::new(Some(Startup { logs, shutdown }));
    let boot = move || {
        let startup = startup
            .lock()
            .expect("GUI startup lock poisoned")
            .take()
            .expect("GUI initialized twice");
        App::new(startup)
    };

    iced::daemon(boot, App::update, App::view)
        .title(APP_NAME)
        .theme(App::theme)
        .subscription(App::subscription)
        .run()
        .context("GUI failed")
}

struct Startup {
    logs: LogReceiver,
    shutdown: watch::Sender<bool>,
}

struct App {
    shutdown: watch::Sender<bool>,
    output: String,
    window: Option<window::Id>,
    _tray: Tray,
}

#[derive(Debug, Clone)]
enum Message {
    Log(String),
    Tray(TrayAction),
    WindowResized(window::Id),
    Minimized(window::Id, Option<bool>),
    WindowClosed(window::Id),
}

#[derive(Debug, Clone, Copy)]
enum TrayAction {
    Show,
    Exit,
}

impl App {
    fn new(startup: Startup) -> (Self, Task<Message>) {
        let (tray, tray_events) = Tray::new();

        (
            Self {
                shutdown: startup.shutdown,
                output: String::new(),
                window: None,
                _tray: tray,
            },
            Task::batch([
                Task::run(startup.logs, Message::Log),
                Task::run(tray_events, Message::Tray),
            ]),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Log(line) => {
                self.output.push_str(&line);
                trim_log(&mut self.output);
                Task::none()
            }
            Message::Tray(TrayAction::Show) => self.show_window(),
            Message::Tray(TrayAction::Exit) => {
                let _ = self.shutdown.send(true);
                iced::exit()
            }
            Message::WindowResized(id) => {
                window::is_minimized(id).map(move |state| Message::Minimized(id, state))
            }
            Message::Minimized(id, Some(true)) if self.window == Some(id) => window::close(id),
            Message::Minimized(_, _) => Task::none(),
            Message::WindowClosed(id) => {
                if self.window == Some(id) {
                    self.window = None;
                }
                Task::none()
            }
        }
    }

    fn show_window(&mut self) -> Task<Message> {
        if let Some(id) = self.window {
            return window::gain_focus(id);
        }

        let (id, open) = window::open(log_window_settings());
        self.window = Some(id);
        open.discard()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            window::resize_events().map(|(id, _)| Message::WindowResized(id)),
            window::close_events().map(Message::WindowClosed),
        ])
    }

    fn view(&self, _window: window::Id) -> Element<'_, Message> {
        container(
            scrollable(
                container(text(&self.output).font(Font::MONOSPACE).size(13))
                    .padding(12)
                    .width(Length::Fill),
            )
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn theme(&self, _window: window::Id) -> Theme {
        Theme::Dark
    }
}

struct Tray {
    _icon: TrayIcon,
    _exit_item: MenuItem,
}

impl Tray {
    fn new() -> (Self, UnboundedReceiver<TrayAction>) {
        let (sender, events) = mpsc::unbounded();

        let show_sender = sender.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                let _ = show_sender.unbounded_send(TrayAction::Show);
            }
        }));

        let menu = Menu::new();
        let exit_item = MenuItem::new("終了", true, None);
        menu.append(&exit_item).expect("failed to build tray menu");

        let exit_id = exit_item.id().clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if event.id == exit_id {
                let _ = sender.unbounded_send(TrayAction::Exit);
            }
        }));

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip(APP_NAME)
            .with_icon(create_tray_icon())
            .build()
            .expect("failed to create tray icon");

        (
            Self {
                _icon: icon,
                _exit_item: exit_item,
            },
            events,
        )
    }
}

fn log_window_settings() -> window::Settings {
    window::Settings {
        size: iced::Size::new(800.0, 480.0),
        icon: Some(
            window::icon::from_rgba(icon_rgba(), 32, 32).expect("failed to create window icon"),
        ),
        ..window::Settings::default()
    }
}

fn icon_rgba() -> Vec<u8> {
    const SIZE: u32 = 32;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as i32 - 15;
            let dy = y as i32 - 15;
            let bridge = (8..=23).contains(&x) && ((9..=13).contains(&y) || (18..=22).contains(&y));
            rgba.extend_from_slice(if bridge {
                &[255, 255, 255, 255]
            } else if dx * dx + dy * dy <= 14 * 14 {
                &[42, 112, 219, 255]
            } else {
                &[0, 0, 0, 0]
            });
        }
    }
    rgba
}

fn create_tray_icon() -> tray_icon::Icon {
    tray_icon::Icon::from_rgba(icon_rgba(), 32, 32).expect("failed to create tray icon image")
}

fn trim_log(output: &mut String) {
    if output.len() <= MAX_LOG_BYTES {
        return;
    }
    let mut start = output.len() - MAX_LOG_BYTES;
    while !output.is_char_boundary(start) {
        start += 1;
    }
    output.drain(..start);
}

fn hide_owned_console() {
    let mut processes = [0_u32; 2];
    let count = unsafe { GetConsoleProcessList(processes.as_mut_ptr(), processes.len() as u32) };
    if count == 1 {
        let window = unsafe { GetConsoleWindow() };
        if !window.is_null() {
            unsafe { ShowWindow(window, SW_HIDE) };
        }
    }
}

#[derive(Clone)]
struct LogWriter(UnboundedSender<String>);

impl Write for LogWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let _ = self
            .0
            .unbounded_send(String::from_utf8_lossy(buffer).into_owned());
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
