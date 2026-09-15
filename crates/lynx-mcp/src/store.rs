use std::collections::{HashMap, VecDeque};

use serde::Serialize;
use tokio::sync::RwLock;

use crate::model::CapturedExchange;

#[derive(Debug, Clone, Copy)]
pub struct StoreLimits {
    pub max_requests: usize,
    pub max_inflight: usize,
    pub max_memory_bytes: usize,
    pub max_body_bytes: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreStats {
    pub inflight: usize,
    pub completed: usize,
    pub estimated_memory_bytes: usize,
    pub oldest_sequence: Option<u64>,
    pub newest_sequence: Option<u64>,
}

#[derive(Default)]
struct Inner {
    inflight: HashMap<String, CapturedExchange>,
    completed: VecDeque<CapturedExchange>,
    next_sequence: u64,
    estimated_memory_bytes: usize,
}

pub struct TrafficStore {
    limits: StoreLimits,
    inner: RwLock<Inner>,
}

impl TrafficStore {
    pub fn new(limits: StoreLimits) -> Self {
        Self {
            limits,
            inner: RwLock::new(Inner {
                next_sequence: 1,
                ..Inner::default()
            }),
        }
    }

    pub fn body_limit(&self) -> usize {
        self.limits.max_body_bytes
    }

    pub async fn update<F>(&self, trace_id: &str, timestamp: i64, update: F)
    where
        F: FnOnce(&mut CapturedExchange),
    {
        let mut inner = self.inner.write().await;
        if !inner.inflight.contains_key(trace_id)
            && inner.inflight.len() >= self.limits.max_inflight
        {
            let oldest = inner
                .inflight
                .iter()
                .min_by_key(|(_, value)| value.started_at)
                .map(|(key, _)| key.clone());
            if let Some(oldest) = oldest
                && let Some(mut exchange) = inner.inflight.remove(&oldest)
            {
                exchange.incomplete = true;
                exchange.finish(timestamp, crate::model::ExchangeState::Incomplete);
                push_completed(&mut inner, exchange, self.limits);
            }
        }
        let exchange = inner
            .inflight
            .entry(trace_id.to_owned())
            .or_insert_with(|| CapturedExchange::placeholder(trace_id.to_owned(), timestamp));
        update(exchange);
    }

    pub async fn complete(&self, trace_id: &str, timestamp: i64) {
        let mut inner = self.inner.write().await;
        let Some(mut exchange) = inner.inflight.remove(trace_id) else {
            return;
        };
        if exchange.completed_at.is_none() {
            exchange.finish(timestamp, crate::model::ExchangeState::Completed);
        }
        push_completed(&mut inner, exchange, self.limits);
    }

    pub async fn mark_gap(&self) {
        let mut inner = self.inner.write().await;
        for exchange in inner.inflight.values_mut() {
            exchange.incomplete = true;
        }
    }

    pub async fn stats(&self) -> StoreStats {
        let inner = self.inner.read().await;
        StoreStats {
            inflight: inner.inflight.len(),
            completed: inner.completed.len(),
            estimated_memory_bytes: inner.estimated_memory_bytes,
            oldest_sequence: inner.completed.front().map(|v| v.sequence),
            newest_sequence: inner.completed.back().map(|v| v.sequence),
        }
    }

    pub async fn get(&self, trace_id: &str) -> Option<CapturedExchange> {
        let inner = self.inner.read().await;
        inner.inflight.get(trace_id).cloned().or_else(|| {
            inner
                .completed
                .iter()
                .find(|v| v.trace_id == trace_id)
                .cloned()
        })
    }

    pub async fn list(
        &self,
        before: Option<u64>,
        limit: usize,
        include_inflight: bool,
    ) -> Vec<CapturedExchange> {
        let inner = self.inner.read().await;
        let mut result: Vec<_> = inner
            .completed
            .iter()
            .rev()
            .filter(|v| before.is_none_or(|cursor| v.sequence < cursor))
            .take(limit)
            .cloned()
            .collect();
        if include_inflight && before.is_none() && result.len() < limit {
            result.extend(inner.inflight.values().take(limit - result.len()).cloned());
        }
        result
    }
}

fn push_completed(inner: &mut Inner, mut exchange: CapturedExchange, limits: StoreLimits) {
    exchange.sequence = inner.next_sequence;
    inner.next_sequence += 1;
    inner.estimated_memory_bytes += exchange.estimated_bytes();
    inner.completed.push_back(exchange);
    while inner.completed.len() > limits.max_requests
        || inner.estimated_memory_bytes > limits.max_memory_bytes
    {
        let Some(removed) = inner.completed.pop_front() else {
            break;
        };
        inner.estimated_memory_bytes = inner
            .estimated_memory_bytes
            .saturating_sub(removed.estimated_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn evicts_oldest_record() {
        let store = TrafficStore::new(StoreLimits {
            max_requests: 1,
            max_inflight: 10,
            max_memory_bytes: 1024,
            max_body_bytes: 32,
        });
        store.update("one", 1, |_| {}).await;
        store.complete("one", 2).await;
        store.update("two", 3, |_| {}).await;
        store.complete("two", 4).await;
        assert!(store.get("one").await.is_none());
        assert!(store.get("two").await.is_some());
    }
}
