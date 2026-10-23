use std::time::Duration;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::android::{Device, LogEntry};

/// Represents all possible events handled by the application event loop.
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Keyboard input event
    Key(KeyEvent),
    /// Mouse input event
    Mouse(MouseEvent),
    /// Terminal resize event (width, height)
    Resize(u16, u16),
    /// Periodic tick for UI animations and ADB status refresh
    Tick,
    /// Line of stdout/stderr from a running subprocess (e.g., Gradle)
    ProcessOutput(String),
    /// Subprocess finished with success status and summary message
    ProcessFinished(bool, String),
    /// Single parsed log entry from Logcat stream
    LogcatLine(LogEntry),
    /// Updated list of devices discovered by ADB poller
    DevicesUpdated(Vec<Device>),
}

/// Asynchronous event handler that aggregates terminal events, ticks,
/// and background worker events into a single Tokio unbounded channel.
pub struct EventHandler {
    sender: UnboundedSender<AppEvent>,
    receiver: UnboundedReceiver<AppEvent>,
    _event_thread: std::thread::JoinHandle<()>,
    _tick_task: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    /// Creates a new `EventHandler` with the specified tick rate.
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        // 1. Terminal event poller in a dedicated OS thread
        // This ensures crossterm's blocking poll never blocks Tokio async tasks or UI.
        let event_sender = sender.clone();
        let _event_thread = std::thread::spawn(move || {
            loop {
                // Poll with a short timeout to prevent high CPU usage and allow responsive shutdown
                match event::poll(Duration::from_millis(50)) {
                    Ok(true) => match event::read() {
                        Ok(crossterm_event) => {
                            let app_event = match crossterm_event {
                                CrosstermEvent::Key(key) => Some(AppEvent::Key(key)),
                                CrosstermEvent::Mouse(mouse) => Some(AppEvent::Mouse(mouse)),
                                CrosstermEvent::Resize(w, h) => Some(AppEvent::Resize(w, h)),
                                _ => None,
                            };
                            if let Some(ev) = app_event {
                                if event_sender.send(ev).is_err() {
                                    break;
                                }
                            }
                        }
                        Err(_) => break,
                    },
                    Ok(false) => {}
                    Err(_) => break,
                }
            }
        });

        // 2. Periodic tick generator
        let tick_sender = sender.clone();
        let _tick_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tick_rate);
            loop {
                interval.tick().await;
                if tick_sender.send(AppEvent::Tick).is_err() {
                    break;
                }
            }
        });

        Self {
            sender,
            receiver,
            _event_thread,
            _tick_task,
        }
    }

    /// Obtains a clone of the event sender for background workers/subprocesses.
    pub fn sender(&self) -> UnboundedSender<AppEvent> {
        self.sender.clone()
    }

    /// Asynchronously receives the next event from the queue.
    pub async fn next(&mut self) -> Option<AppEvent> {
        self.receiver.recv().await
    }
}
