// Simple policy for the containers in the daemon
// Always pull the latest image since I dont know how to implement versioning yet
// For the rest it speaks for itself
// Pull if missing (otherwise how tf can we deploy with it???)
// Use the cached version if its there.
// Since I dont want to keep wasting bandwidth

#[allow(dead_code)]
pub enum ImagePolicy {
    UseCached,
    PullIfMissing,
    AlwaysPullLatest,
}