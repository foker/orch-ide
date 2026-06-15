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

    fn apply(&mut self, cmd: Command) -> Action {
        let mut action = Action::default();
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
        action
    }

    /// Foreground terminal: apply + sync immediately on every event.
    ///
    /// No throttling: the old 60 fps throttle dropped the *trailing* sync when a
    /// burst ended inside the 16 ms window, leaving the last chunk of output
    /// unrendered until the next (possibly never) event — that read as lag /
    /// stale frames. The throttle existed because every terminal synced; now
    /// `handle_quiet` keeps background terminals out of the sync path entirely,
    /// so only the single focused terminal syncs per event and the per-event
    /// cost is bounded. Font/theme changes additionally re-shape the font.
    pub fn handle(&mut self, cmd: Command) -> Action {
        let full = matches!(&cmd, Command::ChangeTheme(_) | Command::ChangeFont(_));
        let action = self.apply(cmd);
        if full {
            self.sync_font();
        }
        self.backend.sync();
        self.redraw();
        action
    }

    /// Apply a command WITHOUT syncing the render grid or clearing the draw
    /// cache. Used for background (non-focused) terminals: keeps the PTY
    /// responsive (PtyWrite/Exit still processed) and drains the event, but
    /// avoids the expensive grid sync + GPU redraw entirely. Call `sync()` when
    /// the terminal is foregrounded to refresh its rendered grid.
    pub fn handle_quiet(&mut self, cmd: Command) -> Action {
        self.apply(cmd)
    }

    /// Force a full grid-sync + redraw (e.g. when a background terminal is
    /// foregrounded after being handled quietly).
    pub fn sync(&mut self) {
        self.sync_font();
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
