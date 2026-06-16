use std::thread;

use sysinfo::System;

use crate::domain::types::ModelRecommendation;

const GIB: u64 = 1024 * 1024 * 1024;
const CPU_HIGHER_RESOURCE_MINIMUM: usize = 6;
const MEMORY_HIGHER_RESOURCE_MINIMUM: u64 = 16 * GIB;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ModelRecommendationInput {
    pub(crate) cuda_available: bool,
    pub(crate) total_memory_bytes: Option<u64>,
    pub(crate) cpu_count: Option<usize>,
}

pub(crate) fn current_model_recommendation(
    cuda_available: bool,
    current_model: &str,
    persisted_model: bool,
) -> ModelRecommendation {
    model_recommendation(
        system_recommendation_input(cuda_available),
        current_model,
        persisted_model,
    )
}

pub(crate) fn model_recommendation(
    input: ModelRecommendationInput,
    current_model: &str,
    persisted_model: bool,
) -> ModelRecommendation {
    let (recommended_model, reason) = recommended_model(input);

    ModelRecommendation {
        recommended_model: recommended_model.to_owned(),
        reason,
        user_overridden: persisted_model && current_model != recommended_model,
    }
}

fn recommended_model(input: ModelRecommendationInput) -> (&'static str, String) {
    if input.cuda_available {
        return (
            "distil-large-v3",
            "Validated CUDA support is available, so distil-large-v3 balances quality and GPU speed"
                .to_owned(),
        );
    }

    match (input.total_memory_bytes, input.cpu_count) {
        (Some(memory), Some(cpu))
            if memory >= MEMORY_HIGHER_RESOURCE_MINIMUM && cpu >= CPU_HIGHER_RESOURCE_MINIMUM =>
        {
            (
                "medium",
                format!(
                    "CPU-only system has {cpu} logical CPUs and {} GiB RAM, so medium is safe",
                    memory / GIB
                ),
            )
        }
        (Some(memory), Some(cpu)) => (
            "small",
            format!(
                "CPU-only system has {cpu} logical CPUs and {} GiB RAM, so small is safer",
                memory / GIB
            ),
        ),
        _ => (
            "small",
            "CPU-only hardware details are incomplete, so small is the safest first-run model"
                .to_owned(),
        ),
    }
}

fn system_recommendation_input(cuda_available: bool) -> ModelRecommendationInput {
    let mut system = System::new();
    system.refresh_memory();

    ModelRecommendationInput {
        cuda_available,
        total_memory_bytes: Some(system.total_memory()),
        cpu_count: thread::available_parallelism().ok().map(usize::from),
    }
}
