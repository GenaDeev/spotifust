#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    TogglePlayPause,
    SkipNext,
    SkipPrev,
    ToggleWindowVisibility,
    Quit,
}

#[allow(dead_code)]
pub struct SystemTrayManager {
    pub is_window_visible: bool,
}

impl Default for SystemTrayManager {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl SystemTrayManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            is_window_visible: true,
        }
    }

    pub fn handle_action(&mut self, action: TrayAction) {
        match action {
            TrayAction::TogglePlayPause | TrayAction::SkipNext | TrayAction::SkipPrev => {}
            TrayAction::ToggleWindowVisibility => {
                self.is_window_visible = !self.is_window_visible;
            }
            TrayAction::Quit => {
                std::process::exit(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_tray_manager_initial_state() {
        let tray = SystemTrayManager::new();
        assert!(tray.is_window_visible);
    }

    #[test]
    fn test_toggle_window_visibility() {
        let mut tray = SystemTrayManager::new();
        tray.handle_action(TrayAction::ToggleWindowVisibility);
        assert!(!tray.is_window_visible);
        tray.handle_action(TrayAction::ToggleWindowVisibility);
        assert!(tray.is_window_visible);
    }
}
