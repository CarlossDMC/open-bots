//! Test doubles shared by provider adapter tests. Nothing here starts a real process.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::{
    error::AppResult,
    infrastructure::process::{LineCommand, LineProcessExit, LineProcessRunner},
    runtime::cancellation::CancellationSignal,
};

/// Replays scripted stdout lines and exits, one response per command in order, and records
/// every command it receives. Commands past the script finish successfully with no output.
pub struct ScriptedRunner {
    responses: Mutex<Vec<(Vec<&'static str>, LineProcessExit)>>,
    pub commands: Mutex<Vec<LineCommand>>,
}

impl ScriptedRunner {
    pub fn new(responses: Vec<(Vec<&'static str>, LineProcessExit)>) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses),
            commands: Mutex::new(Vec::new()),
        })
    }

    pub fn single(lines: Vec<&'static str>, exit: LineProcessExit) -> Arc<Self> {
        Self::new(vec![(lines, exit)])
    }

    pub fn commands(&self) -> Vec<LineCommand> {
        self.commands.lock().expect("commands").clone()
    }
}

#[async_trait]
impl LineProcessRunner for ScriptedRunner {
    async fn run(
        &self,
        command: LineCommand,
        on_line: &mut (dyn for<'line> FnMut(&'line str) + Send),
        _cancellation: CancellationSignal,
    ) -> AppResult<LineProcessExit> {
        self.commands.lock().expect("commands").push(command);
        let (lines, exit) = {
            let mut responses = self.responses.lock().expect("responses");
            if responses.is_empty() {
                (Vec::new(), finished(true))
            } else {
                responses.remove(0)
            }
        };
        for line in lines {
            on_line(line);
        }
        Ok(exit)
    }
}

pub fn finished(success: bool) -> LineProcessExit {
    LineProcessExit::Finished {
        success,
        code: Some(if success { 0 } else { 1 }),
        stderr_tail: String::new(),
    }
}
