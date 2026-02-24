use executor_core::config::{Config, ExecutorConfig, ExecutorType};
use executor_core::error::ExecutorError;
use executor_core::Executor;

/// Create an executor instance from config by name.
pub fn create_executor(
    config: &Config,
    executor_name: &str,
) -> Result<Box<dyn Executor>, ExecutorError> {
    let exec_config = config
        .find_executor(executor_name)
        .ok_or_else(|| ExecutorError::ExecutorNotFound(executor_name.to_string()))?;

    create_executor_from_config(exec_config.clone())
}

/// Create an executor from an ExecutorConfig.
pub fn create_executor_from_config(
    exec_config: ExecutorConfig,
) -> Result<Box<dyn Executor>, ExecutorError> {
    match exec_config.executor_type {
        ExecutorType::Ssh => Ok(Box::new(executor_ssh::SshExecutor::new(exec_config))),
        ExecutorType::Container => Ok(Box::new(
            executor_container::ContainerExecutor::new(exec_config),
        )),
        ExecutorType::Local => Ok(Box::new(executor_local::LocalExecutor::new(exec_config))),
    }
}
