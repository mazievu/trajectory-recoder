use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HealthStatus {
    Healthy,       // Fully operational
    Degraded,      // Minor issues (UIA timeout, extension disconnect, disk Tier 1/2)
    Critical,      // Severe backpressure, disk Tier 3, event drops occurring
    Fatal,         // Subsystem stopped, encryption key lost, crash loop
    Recovering,    // Startup crash recovery in progress
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub probe_name: String,
    pub status: HealthStatus,
    pub message: String,
    pub checked_at: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub trait HealthProbe: Send + Sync {
    fn name(&self) -> &'static str;
    fn check(&self) -> ProbeResult;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthReport {
    pub overall_status: HealthStatus,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub machine_id: String,
    pub process_name: String,
    pub probe_results: Vec<ProbeResult>,
}

pub struct SystemHealthAggregator {
    machine_id: String,
    process_name: String,
    probes: RwLock<Vec<Box<dyn HealthProbe>>>,
}

impl SystemHealthAggregator {
    pub fn new(machine_id: impl Into<String>, process_name: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            machine_id: machine_id.into(),
            process_name: process_name.into(),
            probes: RwLock::new(Vec::new()),
        })
    }

    pub fn register_probe(&self, probe: Box<dyn HealthProbe>) {
        self.probes.write().push(probe);
    }

    pub fn evaluate(&self) -> SystemHealthReport {
        let probes_guard = self.probes.read();
        let mut probe_results = Vec::with_capacity(probes_guard.len());
        let mut worst_status = HealthStatus::Healthy;

        for probe in probes_guard.iter() {
            let res = probe.check();
            if res.status > worst_status {
                worst_status = res.status;
            }
            probe_results.push(res);
        }

        SystemHealthReport {
            overall_status: worst_status,
            generated_at: chrono::Utc::now(),
            machine_id: self.machine_id.clone(),
            process_name: self.process_name.clone(),
            probe_results,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProbe {
        name: &'static str,
        status: HealthStatus,
    }

    impl HealthProbe for MockProbe {
        fn name(&self) -> &'static str {
            self.name
        }

        fn check(&self) -> ProbeResult {
            ProbeResult {
                probe_name: self.name.to_string(),
                status: self.status,
                message: "Probe executed".to_string(),
                checked_at: chrono::Utc::now(),
                metadata: HashMap::new(),
            }
        }
    }

    #[test]
    fn test_health_aggregator_worst_status() {
        let agg = SystemHealthAggregator::new("m1", "agent");
        agg.register_probe(Box::new(MockProbe {
            name: "disk",
            status: HealthStatus::Healthy,
        }));
        agg.register_probe(Box::new(MockProbe {
            name: "uia",
            status: HealthStatus::Degraded,
        }));

        let report = agg.evaluate();
        assert_eq!(report.overall_status, HealthStatus::Degraded);
        assert_eq!(report.probe_results.len(), 2);
    }
}
