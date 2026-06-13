use crate::actions::Action;
use crate::backend;
use crate::bindings::{Binding, BindingAction, BindingsLayout, InputKind};
use crate::font::TermFont;
use crate::settings::{FontSettings, Settings, ThemeSettings};
use crate::theme::{ColorPalette, Theme};
use crate::AlacrittyEvent;
use iced::futures::stream::BoxStream;
use iced::futures::{SinkExt, StreamExt};
use iced::widget::canvas::Cache;
use iced::Subscription;
use std::hash::{Hash, Hasher};
use std::io::Result;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub enum Event {
    BackendCall(u64, backend::Command),
}

#[derive(Debug, Clone)]
pub enum Command {
    ChangeTheme(Box<ColorPalette>),
    ChangeFont(FontSettings),
    AddBindings(Vec<(Binding<InputKind>, BindingAction)>),
    ProxyToBackend(backend::Command),
}

pub struct Terminal {
    pub id: u64,
    widget_id: iced::widget::Id,
    pub(crate) font: TermFont,
    pub(crate) theme: Theme,
    pub(crate) cache: Cache,
    pub(crate) bindings: BindingsLayout,
    pub(crate) backend: backend::Backend,
    backend_event_rx: Arc<Mutex<Receiver<AlacrittyEvent>>>,
    /// Last time `sync_and_redraw` actually ran. Used to throttle PTY-driven
    /// syncs to ~60 fps so a burst of alacritty events does not trigger a
    /// 16-20 MB grid clone per chunk (multiplied by 30+/s during big output
    /// streams it was burning hundreds of MB/s of memory bandwidth and lagging
    /// the whole machine).
    last_sync_at: Option<std::time::Instant>,
}

impl Terminal {
    pub fn new(id: u64, settings: Settings) -> Result<Self> {
        let (backend_event_tx, backend_event_rx) = mpsc::channel(100);
        let theme = Theme::new(settings.theme);
        let font = TermFont::new(settings.font);

        Ok(Self {
            id,
            widget_id: iced::widget::Id::unique(),
            font,
            theme,
            bindings: BindingsLayout::default(),
            cache: Cache::default(),
            backend: backend::Backend::new(
                id,
                backend_event_tx,
                settings.backend,
            )?,
            backend_event_rx: Arc::new(Mutex::new(backend_event_rx)),
            last_sync_at: None,
        })
    }

    pub fn widget_id(&self) -> &iced::widget::Id {
        &self.widget_id
    }

    pub fn subscription(&self) -> Subscription<Event> {
        let data = TerminalSubscriptionData {
            id: self.id,
            event_receiver: self.backend_event_rx.clone(),
        };

        Subscription::run_with(data, terminal_subscription_stream)
    }

    pub fn handle(&mut self, cmd: Command) -> Action {
        let mut action = Action::default();

        // Decide what kind of post-handle sync this command needs.
        //   - Theme / Font change: full sync (font re-shape + grid clone +
        //     redraw), bypass throttle so settings show up immediately.
        //   - ProxyToBackend(ProcessAlacrittyEvent): throttled. This is the
        //     burst-frequency path — alacritty fires Wakeup on every PTY chunk
        //     during a streaming claude output and a full grid clone (~4 MB
        //     even at the reduced 2K scrollback) per chunk saturates RAM
        //     bandwidth. Coalesce to 60 fps.
        //   - Anything else (user Write / Scroll / Resize / Select / Bindings):
        //     immediate sync, NO throttle. Otherwise user input lands in the
        //     16 ms throttle window and the keystroke is invisible until the
        //     next event fires — that was the "I typed ABCD, only ABC shows
        //     up, then E appears as DE" bug.
        enum SyncKind { FullForce, Throttled, Immediate }
        let sync_kind = match &cmd {
            Command::ChangeTheme(_) | Command::ChangeFont(_) => SyncKind::FullForce,
            Command::ProxyToBackend(backend::Command::ProcessAlacrittyEvent(_)) =>
                SyncKind::Throttled,
            _ => SyncKind::Immediate,
        };

        match cmd {
            Command::ChangeTheme(color_pallete) => {
                self.theme = Theme::new(ThemeSettings::new(color_pallete));
            },
            Command::ChangeFont(font_settings) => {
                self.font = TermFont::new(font_settings);
            },
            Command::AddBindings(bindings) => {
                self.bindings.add_bindings(bindings);
            },
            Command::ProxyToBackend(cmd) => {
                action = self.backend.handle(cmd);
            },
        };

        match sync_kind {
            SyncKind::FullForce => {
                self.sync_font();
                self.backend.sync();
                self.redraw();
                self.last_sync_at = Some(std::time::Instant::now());
            },
            SyncKind::Immediate => {
                self.backend.sync();
                self.redraw();
                self.last_sync_at = Some(std::time::Instant::now());
            },
            SyncKind::Throttled => {
                self.throttled_sync_and_redraw();
            },
        }
        action
    }

    /// Throttle grid-sync + canvas-redraw to ~60 fps. Each `backend.sync()`
    /// clones the full alacritty Grid (~20 MB at 10K scrollback); calling it
    /// 100-300 times per second during a claude streaming burst was the main
    /// cause of system-wide lag (RAM bandwidth saturation + lock contention
    /// with alacritty's PTY EventLoop). Skipping when <16 ms passed since the
    /// last sync coalesces bursts into one redraw per frame. Trailing events
    /// in a burst still get rendered: the next event after the throttle
    /// window will sync.
    fn throttled_sync_and_redraw(&mut self) {
        const MIN_INTERVAL: std::time::Duration =
            std::time::Duration::from_millis(16);
        let now = std::time::Instant::now();
        if let Some(last) = self.last_sync_at {
            if now.duration_since(last) < MIN_INTERVAL {
                return;
            }
        }
        self.last_sync_at = Some(now);
        self.backend.sync();
        self.redraw();
    }

    fn sync_font(&mut self) {
        self.font.sync();
        self.backend
            .handle(backend::Command::Resize(None, Some(self.font.measure)));
    }

    fn redraw(&mut self) {
        self.cache.clear();
    }
}

#[derive(Clone)]
struct TerminalSubscriptionData {
    id: u64,
    event_receiver: Arc<Mutex<Receiver<AlacrittyEvent>>>,
}

impl Hash for TerminalSubscriptionData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

fn terminal_subscription_stream(
    data: &TerminalSubscriptionData,
) -> BoxStream<'static, Event> {
    let id = data.id;
    let event_receiver = data.event_receiver.clone();
    iced::stream::channel(1000, async move |mut output| {
        let mut shutdown = false;
        loop {
            let mut event_receiver = event_receiver.lock().await;
            match event_receiver.recv().await {
                Some(event) => {
                    if let AlacrittyEvent::Exit = event {
                        shutdown = true;
                    }

                    if output
                        .send(Event::BackendCall(id, backend::Command::ProcessAlacrittyEvent(event)))
                        .await
                        .is_err()
                    {
                        // Subscriber went away (app shutting down / terminal dropped).
                        // Exit the loop instead of panicking — drops the receiver lock
                        // so the backend mutex can be cleaned up.
                        eprintln!("iced_term stream {}: subscriber gone, exiting", id);
                        break;
                    }
                },
                None => {
                    // Channel closed. Always break (whether shutdown was signalled or
                    // not) — looping on None is a busy-loop that holds the receiver
                    // lock forever and was a deadlock source. Panicking on
                    // unexpected-close also took the whole tokio worker down and left
                    // dangling backend state, causing the UI-thread mutex deadlock.
                    if !shutdown {
                        eprintln!("iced_term stream {}: terminal event channel closed unexpected", id);
                    }
                    break;
                },
            }
        }
    })
    .boxed()
}
