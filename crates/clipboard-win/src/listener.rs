use std::{
    sync::mpsc::{Receiver, sync_channel},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{ClipboardError, ClipboardNotification, NotificationReceiveError, sys};

pub struct ClipboardNotifications {
    receiver: Receiver<ClipboardNotification>,
}

impl ClipboardNotifications {
    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<ClipboardNotification, NotificationReceiveError> {
        self.receiver.recv_timeout(timeout).map_err(Into::into)
    }

    pub fn try_recv(&self) -> Result<ClipboardNotification, NotificationReceiveError> {
        self.receiver.try_recv().map_err(Into::into)
    }
}

pub struct ClipboardListener {
    endpoint: Option<sys::ListenerEndpoint>,
    thread: Option<JoinHandle<Result<(), ClipboardError>>>,
}

impl ClipboardListener {
    pub fn start() -> Result<(Self, ClipboardNotifications), ClipboardError> {
        let (notification_sender, notification_receiver) = sync_channel(1);
        let (startup_sender, startup_receiver) = sync_channel(1);
        let thread = thread::Builder::new()
            .name("pastral-clipboard-listener".to_owned())
            .spawn(move || sys::run_listener(notification_sender, startup_sender))
            .map_err(|_| ClipboardError::ListenerThreadSpawn)?;

        let startup = match startup_receiver.recv() {
            Ok(value) => value,
            Err(_) => {
                let _ = thread.join();
                return Err(ClipboardError::ListenerStartupClosed);
            }
        };
        match startup {
            Ok(endpoint) => Ok((
                Self {
                    endpoint: Some(endpoint),
                    thread: Some(thread),
                },
                ClipboardNotifications {
                    receiver: notification_receiver,
                },
            )),
            Err(error) => {
                let _ = thread.join();
                Err(error)
            }
        }
    }

    pub fn stop(mut self) -> Result<(), ClipboardError> {
        self.stop_inner()
    }

    fn stop_inner(&mut self) -> Result<(), ClipboardError> {
        if let Some(endpoint) = self.endpoint.take() {
            match sys::post_listener_stop(endpoint) {
                Ok(()) => {}
                Err(error) => {
                    self.endpoint = Some(endpoint);
                    return Err(error);
                }
            }
        }
        if let Some(thread) = self.thread.take() {
            return thread
                .join()
                .map_err(|_| ClipboardError::ListenerThreadPanicked)?;
        }
        Ok(())
    }

    #[cfg(test)]
    fn post_test_update(&self) -> Result<(), ClipboardError> {
        sys::post_listener_test_update(self.endpoint.ok_or(ClipboardError::ListenerStartupClosed)?)
    }
}

impl Drop for ClipboardListener {
    fn drop(&mut self) {
        let _ = self.stop_inner();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::TryRecvError;

    use super::*;

    #[test]
    fn listener_receives_posted_update_without_mutating_clipboard() {
        let (listener, notifications) = ClipboardListener::start().unwrap();
        while !matches!(
            notifications.try_recv(),
            Err(NotificationReceiveError::Empty)
        ) {}
        listener.post_test_update().unwrap();
        let notification = notifications.recv_timeout(Duration::from_secs(2)).unwrap();
        let _ = notification.sequence();
        listener.stop().unwrap();
    }

    #[test]
    fn bounded_notifications_coalesce_without_blocking_window_thread() {
        let (listener, notifications) = ClipboardListener::start().unwrap();
        while !matches!(
            notifications.try_recv(),
            Err(NotificationReceiveError::Empty)
        ) {}
        for _ in 0..32 {
            listener.post_test_update().unwrap();
        }
        assert!(notifications.recv_timeout(Duration::from_secs(2)).is_ok());
        listener.stop().unwrap();
    }

    #[test]
    fn repeated_start_and_stop_releases_each_window() {
        for _ in 0..3 {
            let (listener, _notifications) = ClipboardListener::start().unwrap();
            listener.stop().unwrap();
        }
    }

    #[test]
    fn disconnected_receive_is_distinct() {
        let (_sender, receiver) = sync_channel::<ClipboardNotification>(1);
        drop(_sender);
        assert_eq!(
            ClipboardNotifications { receiver }.try_recv(),
            Err(NotificationReceiveError::Disconnected)
        );
        assert_eq!(TryRecvError::Disconnected, TryRecvError::Disconnected);
    }
}
