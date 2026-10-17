//! Fixture for V-INV-08 (SPR-012 T-07): a death count drawn from a Poisson sampler instead of one
//! Bernoulli per person. bernoulli_only in bin/check_all.sh must FAIL on this directory.
pub fn deaths(stream: &mut rh_ids::RngStream, mean: u64) -> u64 {
    rh_sample::poisson(stream, mean)
}
