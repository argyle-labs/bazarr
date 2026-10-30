//! Bazarr's `ServiceBackend` — deploy/backup/restore of the Bazarr instance
//! itself. Bazarr is not Servarr-shaped (its own API, not the shared *arr
//! `X-Api-Key` client), so this plugin carries no typed API client and no
//! `#[orca_tool]` surface — service-backend only.

use plugin_toolkit::service::{
    BoxFuture, Routes, Runtime, ServiceBackend, ServiceCapability, ServiceError, ServiceStatus,
    WorkloadSpec,
};

#[derive(Debug, Clone)]
pub struct BazarrServiceBackend {
    provider: &'static str,
}

impl BazarrServiceBackend {
    pub fn new(provider: &'static str) -> Self {
        Self { provider }
    }
}

impl ServiceBackend for BazarrServiceBackend {
    fn provider(&self) -> &str {
        self.provider
    }

    fn runtimes(&self) -> Vec<Runtime> {
        vec![Runtime::Docker, Runtime::Podman, Runtime::Lxc, Runtime::Vm]
    }

    fn capabilities(&self) -> Vec<ServiceCapability> {
        vec![
            ServiceCapability::Deploy,
            ServiceCapability::Backup,
            ServiceCapability::Restore,
            ServiceCapability::Configure,
            ServiceCapability::Status,
        ]
    }

    fn default_port(&self) -> u16 {
        6767
    }

    /// `/config` is the only persistent state; `/data/media` is a bulk/
    /// operational mount excluded from backup.
    fn data_paths(&self) -> Vec<String> {
        vec!["/config".to_string()]
    }

    fn workload_spec<'a>(
        &'a self,
        _runtime: Runtime,
        _instance: &'a str,
        _routes: &'a Routes,
    ) -> BoxFuture<'a, Result<WorkloadSpec, ServiceError>> {
        Box::pin(async move { Err(ServiceError::unimplemented("bazarr.workload_spec")) })
    }

    fn configure<'a>(
        &'a self,
        _instance: &'a str,
        _routes: &'a Routes,
        _config: &'a str,
    ) -> BoxFuture<'a, Result<(), ServiceError>> {
        Box::pin(async move { Err(ServiceError::unimplemented("bazarr.configure")) })
    }

    fn status<'a>(
        &'a self,
        _instance: &'a str,
        _routes: &'a Routes,
    ) -> BoxFuture<'a, Result<ServiceStatus, ServiceError>> {
        Box::pin(async move { Err(ServiceError::unimplemented("bazarr.status")) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_backend_declares_config_only() {
        let b = BazarrServiceBackend::new("bazarr");
        assert_eq!(b.provider(), "bazarr");
        assert_eq!(b.default_port(), 6767);
        assert_eq!(b.data_paths(), vec!["/config".to_string()]);
    }
}
