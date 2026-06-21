use std::collections::HashSet;

use lingmo_core_engine::error::BuildResult;
use lingmo_core_engine::types::BuildContext;
use lingmo_plugins::registry::PluginRegistry;

use crate::stage::Stage;
use crate::stage_executor::StageExecutor;

pub struct Pipeline {
    executor: StageExecutor,
    skip_stages: HashSet<Stage>,
    only_stages: Option<Vec<Stage>>,
}

impl Pipeline {
    pub fn new(ctx: BuildContext, registry: PluginRegistry) -> Self {
        let executor = StageExecutor::new(ctx, registry);
        Pipeline {
            executor,
            skip_stages: HashSet::new(),
            only_stages: None,
        }
    }

    pub fn with_skip_stages(mut self, stages: Vec<Stage>) -> Self {
        self.skip_stages = stages.into_iter().collect();
        self
    }

    pub fn with_only_stages(mut self, stages: Vec<Stage>) -> Self {
        self.only_stages = Some(stages);
        self
    }

    pub fn execute(&self) -> BuildResult<()> {
        let stages = self.resolve_stages();
        let total = stages.len();
        let start = std::time::Instant::now();

        for (i, stage) in stages.iter().enumerate() {
            let progress = format!("[{}/{}]", i + 1, total);
            tracing::info!("{} Executing stage: {}", progress, stage);

            let stage_start = std::time::Instant::now();
            if let Err(e) = self.executor.execute(*stage) {
                tracing::error!(
                    "Stage '{}' failed after {:?}: {}",
                    stage,
                    stage_start.elapsed(),
                    e
                );
                return Err(e);
            }
            tracing::info!(
                "{} Stage '{}' completed in {:?}",
                progress,
                stage,
                stage_start.elapsed()
            );
        }

        tracing::info!(
            "Pipeline completed successfully in {:?}",
            start.elapsed()
        );
        Ok(())
    }

    fn resolve_stages(&self) -> Vec<Stage> {
        let filtered: Vec<Stage> = Stage::ALL
            .iter()
            .filter(|s| !self.skip_stages.contains(s))
            .copied()
            .collect();

        let stages_to_run = match &self.only_stages {
            Some(only) => {
                let only_set: HashSet<Stage> = only.iter().copied().collect();
                filtered.into_iter().filter(|s| only_set.contains(s)).collect()
            }
            None => filtered,
        };

        // Maintain the original order
        Stage::ALL
            .iter()
            .filter(|s| stages_to_run.contains(s))
            .copied()
            .collect()
    }
}
