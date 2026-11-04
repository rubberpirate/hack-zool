//! Reproducible local PostgreSQL microbenchmarks, not a blockchain throughput benchmark.
use serde_json::{Value, json};
use std::{hint::black_box, time::Instant};
use uuid::Uuid;
use zsplit_claims::TokenService;
use zsplit_integration::Fixture;

fn distribution(mut samples: Vec<f64>) -> Value {
    samples.sort_by(f64::total_cmp);
    let percentile = |p: f64| {
        samples[((samples.len() as f64 * p).ceil() as usize)
            .saturating_sub(1)
            .min(samples.len() - 1)]
    };
    json!({"samples":samples.len(),"p50_ms":percentile(0.50),"p95_ms":percentile(0.95),"p99_ms":percentile(0.99),"min_ms":samples[0],"max_ms":samples[samples.len()-1],"raw_ms":samples})
}
#[tokio::main]
async fn main() {
    let iterations = std::env::var("BENCH_ITERATIONS")
        .ok()
        .map(|s| s.parse::<usize>().expect("BENCH_ITERATIONS integer"))
        .unwrap_or(20);
    assert!(iterations >= 2, "at least two samples required");
    let tokens = TokenService::new([61; 32]);
    let mut token_samples = Vec::new();
    for _ in 0..iterations * 10 {
        let start = Instant::now();
        for _ in 0..50 {
            black_box(tokens.issue());
        }
        token_samples.push(start.elapsed().as_secs_f64() * 1_000.0);
    }
    let mut issuance_samples = Vec::new();
    let mut enqueue_samples = Vec::new();
    let mut lease_samples = Vec::new();
    let mut summary_samples = Vec::new();
    for _ in 0..iterations {
        let fixture = Fixture::new(50).await;
        let start = Instant::now();
        fixture.issue().await;
        issuance_samples.push(start.elapsed().as_secs_f64() * 1_000.0);
        let start = Instant::now();
        for index in 0..50 {
            fixture
                .store
                .enqueue_redemption(fixture.redeem(index, Uuid::new_v4()))
                .await
                .unwrap();
        }
        enqueue_samples.push(start.elapsed().as_secs_f64() * 1_000.0);
        let worker = Uuid::new_v4();
        let start = Instant::now();
        for _ in 0..50 {
            let job = fixture
                .store
                .lease_job(worker)
                .await
                .unwrap()
                .expect("one queued intent");
            fixture
                .store
                .mark_manual_review(job.id, worker)
                .await
                .unwrap();
        }
        lease_samples.push(start.elapsed().as_secs_f64() * 1_000.0);
        for _ in 0..20 {
            let start = Instant::now();
            black_box(
                fixture
                    .store
                    .summary(fixture.room, fixture.owner)
                    .await
                    .unwrap(),
            );
            summary_samples.push(start.elapsed().as_secs_f64() * 1_000.0);
        }
        fixture.finish().await;
    }
    let memory = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|line| line.starts_with("VmHWM:"))
                .map(str::to_string)
        });
    println!("{}",serde_json::to_string_pretty(&json!({
      "kind":"PostgreSQL application benchmark with synthetic accounting fixtures, no blockchain funding or proof generation",
      "profile":if cfg!(debug_assertions){"debug"}else{"release"},"iterations":iterations,"peak_memory_linux":memory,
      "token_generation_50":distribution(token_samples),"postgres_issue_50":distribution(issuance_samples),
      "postgres_enqueue_50":distribution(enqueue_samples),"postgres_lease_and_quarantine_50":distribution(lease_samples),
      "postgres_summary":distribution(summary_samples)
    })).unwrap());
}
