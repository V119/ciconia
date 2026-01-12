use crate::service::tunnel::TunnelService;
use crate::settings::SettingsManager;
use std::sync::Arc;

pub struct AppState {
    pub tunnel_service: Arc<TunnelService>,
    pub settings: SettingsManager,
}

impl AppState {
    pub fn new(tunnel_service: TunnelService, settings: SettingsManager) -> Self {
        let tunnel_service = Arc::new(tunnel_service);
        Self {
            tunnel_service,
            settings,
        }
    }
}
