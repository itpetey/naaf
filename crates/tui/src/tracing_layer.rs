use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::mpsc;
use tracing::Subscriber;
use tracing_subscriber::Layer;

use crate::event::TuiEvent;
use naaf_core::span;

pub struct TuiLayer {
    tx: mpsc::UnboundedSender<TuiEvent>,
    spans: Arc<Mutex<HashMap<u64, SpanInfo>>>,
}

#[derive(Clone, Debug)]
struct SpanInfo {
    name: String,
    component: Option<String>,
    task_name: Option<String>,
}

impl TuiLayer {
    pub fn new(tx: mpsc::UnboundedSender<TuiEvent>) -> Self {
        Self {
            tx,
            spans: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn send(&self, event: TuiEvent) {
        let _ = self.tx.send(event);
    }

    fn find_span_info(
        &self,
        id: &tracing::span::Id,
        spans: &HashMap<u64, SpanInfo>,
    ) -> Option<SpanInfo> {
        spans.get(&id.into_u64()).cloned()
    }
}

impl<S> Layer<S> for TuiLayer
where
    S: Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut component = None;
        let mut task_name = None;

        let mut visitor = SpanFieldVisitor {
            component: &mut component,
            task_name: &mut task_name,
        };
        attrs.values().record(&mut visitor);

        let name = attrs.metadata().name().to_string();

        let mut spans = self.spans.lock();
        spans.insert(
            id.into_u64(),
            SpanInfo {
                name,
                component,
                task_name,
            },
        );
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut action = None;
        let mut attempt: Option<u64> = None;
        let mut accepted = None;
        let mut finding_count: Option<u64> = None;
        let mut reason = None;
        let mut stage = None;
        let mut message = String::new();

        let mut visitor = EventFieldVisitor {
            action: &mut action,
            attempt: &mut attempt,
            accepted: &mut accepted,
            finding_count: &mut finding_count,
            reason: &mut reason,
            stage: &mut stage,
            message: &mut message,
        };
        event.record(&mut visitor);

        let spans = self.spans.lock();

        let current_span = ctx.event_span(event).or_else(|| ctx.lookup_current());
        let span_info =
            current_span
                .as_ref()
                .and_then(|s: &tracing_subscriber::registry::SpanRef<'_, S>| {
                    self.find_span_info(&s.id(), &spans)
                });

        if let Some(info) = &span_info {
            let task = info.task_name.clone().unwrap_or_else(|| info.name.clone());

            match info.name.as_str() {
                span::name::STEP => {
                    if let Some(action) = action.as_deref() {
                        match action {
                            span::action::RUN_START => {
                                self.send(TuiEvent::StepStarted {
                                    task_name: task.clone(),
                                });
                            }
                            span::action::ATTEMPT_START => {
                                self.send(TuiEvent::StepAttemptStarted {
                                    task_name: task.clone(),
                                    attempt: attempt.unwrap_or(0) as usize,
                                });
                            }
                            span::action::ATTEMPT_VALIDATED => {
                                self.send(TuiEvent::StepAttemptValidated {
                                    task_name: task.clone(),
                                    attempt: attempt.unwrap_or(0) as usize,
                                    accepted: accepted.unwrap_or(false),
                                    finding_count: finding_count.unwrap_or(0) as usize,
                                });
                            }
                            span::action::ATTEMPT_REPAIR_START => {
                                self.send(TuiEvent::StepRepairStarted {
                                    task_name: task.clone(),
                                    attempt: attempt.unwrap_or(0) as usize,
                                });
                            }
                            span::action::RUN_COMPLETE => {
                                self.send(TuiEvent::StepCompleted {
                                    task_name: task.clone(),
                                    attempts: attempt.unwrap_or(1) as usize,
                                });
                            }
                            span::action::RUN_REJECTED => {
                                self.send(TuiEvent::StepRejected {
                                    task_name: task.clone(),
                                    attempts: attempt.unwrap_or(0) as usize,
                                    reason: reason.unwrap_or_else(|| "unknown".to_string()),
                                });
                            }
                            span::action::RUN_ERROR => {
                                self.send(TuiEvent::StepFailed {
                                    task_name: task.clone(),
                                    stage: stage.unwrap_or_else(|| "unknown".to_string()),
                                });
                            }
                            _ => {}
                        }
                    }
                }
                span::name::TASK
                | span::name::CHECK
                | span::name::MATERIALISER
                | span::name::REPAIR => {
                    let component = info.component.clone().unwrap_or_default();
                    if let Some(action) = action.as_deref() {
                        match action {
                            span::action::RUN_START => {
                                self.send(TuiEvent::ComponentStarted {
                                    component,
                                    name: task.clone(),
                                });
                            }
                            span::action::RUN_COMPLETE => {
                                self.send(TuiEvent::ComponentCompleted {
                                    component,
                                    name: task.clone(),
                                });
                            }
                            span::action::RUN_ERROR => {
                                self.send(TuiEvent::ComponentFailed {
                                    component,
                                    name: task.clone(),
                                });
                            }
                            _ => {}
                        }
                    }
                }
                _ => {
                    if !message.is_empty() {
                        self.send(TuiEvent::Log {
                            level: *event.metadata().level(),
                            target: event.metadata().target().to_string(),
                            message,
                        });
                    }
                }
            }
        } else if !message.is_empty() {
            self.send(TuiEvent::Log {
                level: *event.metadata().level(),
                target: event.metadata().target().to_string(),
                message,
            });
        }
    }

    fn on_close(&self, id: tracing::span::Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut spans = self.spans.lock();
        spans.remove(&id.into_u64());
    }
}

struct SpanFieldVisitor<'a> {
    component: &'a mut Option<String>,
    task_name: &'a mut Option<String>,
}

impl tracing::field::Visit for SpanFieldVisitor<'_> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "component" => *self.component = Some(value.to_string()),
            "task" | "check" | "materialiser" | "planner" => {
                *self.task_name = Some(value.to_string());
            }
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "component" {
            *self.component = Some(format!("{value:?}"));
        }
    }
}

struct EventFieldVisitor<'a> {
    action: &'a mut Option<String>,
    attempt: &'a mut Option<u64>,
    accepted: &'a mut Option<bool>,
    finding_count: &'a mut Option<u64>,
    reason: &'a mut Option<String>,
    stage: &'a mut Option<String>,
    message: &'a mut String,
}

impl tracing::field::Visit for EventFieldVisitor<'_> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "action" => *self.action = Some(value.to_string()),
            "reason" => *self.reason = Some(value.to_string()),
            "stage" => *self.stage = Some(value.to_string()),
            "message" => *self.message = value.to_string(),
            _ => {}
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if field.name() == "accepted" {
            *self.accepted = Some(value);
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        match field.name() {
            "attempt" => *self.attempt = Some(value),
            "finding_count" => *self.finding_count = Some(value),
            "attempts" => *self.attempt = Some(value),
            _ => {}
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        if value >= 0 {
            match field.name() {
                "attempt" => *self.attempt = Some(value as u64),
                "finding_count" => *self.finding_count = Some(value as u64),
                "attempts" => *self.attempt = Some(value as u64),
                _ => {}
            }
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" || field.name() == "" {
            *self.message = format!("{value:?}");
        }
    }
}
