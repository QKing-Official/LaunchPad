// Simple policy for the containers in the daemon

#[allow(dead_code)]
pub enum ImagePolicy {
    UseCached,
    PullIfMissing,
    AlwaysPullLatest,
}