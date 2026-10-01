use std::sync::Arc;

use serde::Serialize;

use crate::{
    error::{AppError, AppResult},
    providers::{
        AgentProvider, ProviderCapability, ProviderModel, ProviderRegistry, ProviderUsage,
    },
};

/// Usage for one provider, or the reason it could not be read.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageReport {
    pub provider_id: String,
    pub provider_name: String,
    pub usage: Option<ProviderUsage>,
    pub error: Option<String>,
}

/// Reads provider catalogs and account usage through declared capabilities.
pub struct ProviderService {
    providers: Arc<ProviderRegistry>,
}

impl ProviderService {
    pub fn new(providers: Arc<ProviderRegistry>) -> Self {
        Self { providers }
    }

    pub async fn list_models(&self, provider_id: &str) -> AppResult<Vec<ProviderModel>> {
        let provider = self.providers.get(provider_id)?;
        require(
            provider.as_ref(),
            ProviderCapability::ModelSelection,
            "model selection",
        )?;
        provider.list_models().await
    }

    /// Reads every provider that reports usage. One failing provider never hides the others.
    pub async fn read_all_usage(&self) -> Vec<ProviderUsageReport> {
        let mut reports = Vec::new();
        for provider in self.providers.all() {
            if !provider
                .capabilities()
                .contains(&ProviderCapability::UsageLimits)
            {
                continue;
            }
            let (usage, error) = match provider.read_usage().await {
                Ok(usage) => (Some(usage), None),
                Err(error) => {
                    tracing::warn!(provider_id = provider.id(), %error, "provider usage could not be read");
                    (None, Some(error.to_string()))
                }
            };
            reports.push(ProviderUsageReport {
                provider_id: provider.id().into(),
                provider_name: provider.name().into(),
                usage,
                error,
            });
        }
        reports
    }
}

pub(super) fn require(
    provider: &dyn AgentProvider,
    capability: ProviderCapability,
    label: &str,
) -> AppResult<()> {
    if provider.capabilities().contains(&capability) {
        Ok(())
    } else {
        Err(AppError::Unsupported(format!(
            "{} does not support {label}",
            provider.name()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        providers::{
            DetectionStatus, MockProvider, ProviderKind, ProviderSummary, TurnEvents, TurnOutcome,
            TurnRequest, UsageWindow,
        },
        runtime::cancellation::CancellationSignal,
    };
    use async_trait::async_trait;
    use chrono::Utc;

    /// Declares usage limits and either reports fixed usage or fails.
    struct MeteredProvider {
        fails: bool,
    }

    #[async_trait]
    impl AgentProvider for MeteredProvider {
        fn id(&self) -> &'static str {
            if self.fails {
                "broken"
            } else {
                "metered"
            }
        }
        fn name(&self) -> &'static str {
            if self.fails {
                "Broken"
            } else {
                "Metered"
            }
        }
        async fn detect(&self) -> AppResult<ProviderSummary> {
            Ok(ProviderSummary {
                id: self.id().into(),
                name: self.name().into(),
                kind: ProviderKind::Mock,
                status: DetectionStatus::Available,
                detail: String::new(),
                capabilities: self.capabilities(),
            })
        }
        fn capabilities(&self) -> Vec<ProviderCapability> {
            vec![ProviderCapability::UsageLimits]
        }
        async fn run_turn(
            &self,
            _request: TurnRequest,
            _events: TurnEvents,
            _cancellation: CancellationSignal,
        ) -> AppResult<TurnOutcome> {
            Err(AppError::Unsupported("turns".into()))
        }
        async fn read_usage(&self) -> AppResult<ProviderUsage> {
            if self.fails {
                return Err(AppError::Provider("offline".into()));
            }
            Ok(ProviderUsage {
                provider_id: self.id().into(),
                plan: None,
                windows: vec![UsageWindow {
                    duration_minutes: Some(300),
                    used_percent: 40,
                    resets_at: None,
                }],
                limit_reached: false,
                checked_at: Utc::now(),
            })
        }
    }

    fn service() -> ProviderService {
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(MockProvider)).expect("mock");
        registry
            .register(Arc::new(MeteredProvider { fails: false }))
            .expect("metered");
        registry
            .register(Arc::new(MeteredProvider { fails: true }))
            .expect("broken");
        ProviderService::new(Arc::new(registry))
    }

    #[tokio::test]
    async fn reads_usage_only_from_capable_providers_and_keeps_failures_separate() {
        let reports = service().read_all_usage().await;
        let ids: Vec<_> = reports
            .iter()
            .map(|report| report.provider_id.as_str())
            .collect();
        assert_eq!(ids, ["broken", "metered"]);
        assert_eq!(
            reports[0].error.as_deref(),
            Some("provider operation failed: offline")
        );
        assert!(reports[0].usage.is_none());
        assert_eq!(
            reports[1]
                .usage
                .as_ref()
                .map(|usage| usage.windows[0].used_percent),
            Some(40)
        );
    }

    #[tokio::test]
    async fn refuses_model_lists_without_the_capability() {
        let result = service().list_models("mock").await;
        assert!(matches!(result, Err(AppError::Unsupported(_))));
        assert!(matches!(
            service().list_models("missing").await,
            Err(AppError::NotFound(_))
        ));
    }
}
