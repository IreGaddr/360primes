use num_bigint::{BigUint, ToBigUint};
use num_traits::{Zero, One, ToPrimitive};
use primal::Sieve;
use rayon::prelude::*;
use std::env;
use std::time::{Instant, Duration};
use std::cmp::{min, max};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use num_prime::nt_funcs::is_prime;
use num_prime::Primality;

// Keep the proven value for coverage
const MAX_K: u64 = 180;

// Optimization constants
const PRIME_TEST_CHUNK_SIZE: usize = 10_000;
const PARALLEL_SCALE_CHUNK_SIZE: u128 = 10;
const REPORT_INTERVAL: u128 = 1_000;

// Helper for BigUint subtraction that doesn't panic on underflow
trait SaturatingSub {
    fn saturating_sub(&self, other: &Self) -> Self;
}

impl SaturatingSub for BigUint {
    fn saturating_sub(&self, other: &Self) -> Self {
        if self > other {
            self - other
        } else {
            BigUint::zero()
        }
    }
}

// More efficient factorization for large numbers
fn get_factors_biguint(n: &BigUint) -> Vec<BigUint> {
    // For small enough numbers where we can convert to u64, use primal's efficient factorization
    if let Some(n_u64) = n.to_u64() {
        let small_factors = match Sieve::new(n_u64 as usize).factor(n_u64 as usize) {
            Ok(factors) => factors,
            Err((_, factors)) => factors, // If partially factored, use what we have
        };
            
        let mut all_factors = vec![1u64.to_biguint().unwrap()];
        
        // Generate all combinations of prime factors
        for (prime, max_power) in small_factors {
            let prime_biguint = (prime as u64).to_biguint().unwrap();
            let mut new_factors = Vec::new();
            
            for factor in &all_factors {
                let mut current = factor.clone();
                for _ in 0..max_power {
                    current = current * &prime_biguint;
                    new_factors.push(current.clone());
                }
            }
            
            all_factors.extend(new_factors);
        }
        
        all_factors.sort();
        return all_factors;
    }
    
    // For larger numbers, use a more efficient approach for our specific needs
    // For extremely large numbers, we don't need ALL factors, just those near our range
    // Instead, we'll focus on smaller factors which are more relevant for the pattern

    let mut factors = vec![BigUint::one()];
    let two = 2u64.to_biguint().unwrap();
    
    // Only check potential factors up to sqrt(n)
    let limit = n.sqrt() + BigUint::one();
    let mut i = two.clone();
    
    while &i <= &limit {
        if n % &i == BigUint::zero() {
            factors.push(i.clone());
            let quotient = n / &i;
            if &i != &quotient {
                factors.push(quotient);
            }
        }
        i = i + BigUint::one();
    }
    
    // Add the number itself as a factor
    factors.push(n.clone());
    factors.sort();
    factors
}

fn recursive_sequence_generator_optimized(base: &BigUint, max_value: &BigUint) -> Vec<BigUint> {
    let mut terms = Vec::new();
    let mut n_i = base.clone();
    let mut i = BigUint::one();
    
    while &n_i <= max_value {
        terms.push(n_i.clone());
        i += BigUint::one();
        n_i += &i;
    }
    
    terms
}

fn check_proximity_biguint(prime: &BigUint, candidates: &[BigUint], max_k: &BigUint) -> bool {
    for candidate in candidates {
        let diff = if prime > candidate {
            prime - candidate
        } else {
            candidate - prime
        };
        
        if &diff <= max_k {
            return true;
        }
    }
    false
}

fn is_prime_biguint(n: &BigUint) -> bool {
    // Use num_prime for larger numbers
    // Convert small numbers to u64 for faster checking
    if let Some(n_u64) = n.to_u64() {
        if n_u64 <= 1 {
            return false;
        }
        
        // Use primal's is_prime for small numbers (faster)
        if n_u64 <= u32::MAX as u64 {
            let sieve = Sieve::new(min(n_u64 as usize + 1, 10_000_000));
            return sieve.is_prime(n_u64 as usize);
        }
    }
    
    // Use num_prime for larger numbers with default config (None)
    match is_prime(n, None) {
        Primality::Yes => true,
        _ => false
    }
}

fn generate_primes_in_range(range_start: &BigUint, range_end: &BigUint) -> Vec<BigUint> {
    let mut primes = Vec::new();
    
    // If the range is small enough to convert to u64, use primal's efficient sieve
    if let (Some(start_u64), Some(end_u64)) = (range_start.to_u64(), range_end.to_u64()) {
        let sieve = Sieve::new(end_u64 as usize + 1);
        return sieve.primes_from(0)
            .take_while(|&p| p <= end_u64 as usize)
            .filter(|&p| p > start_u64 as usize)
            .map(|p| p.to_biguint().unwrap())
            .collect();
    }
    
    // For larger ranges, use parallel chunks with primality testing
    let range_size = range_end - range_start;
    
    // If range is too large, sample primes instead of checking every number
    if let Some(size) = range_size.to_u64() {
        if size > 1_000_000 {
            println!("  Range is very large ({} to {}). Using sampling approach.", range_start, range_end);
            
            // Start at an odd number
            let mut start = range_start.clone();
            if &start % 2u32 == BigUint::zero() {
                start += BigUint::one();
            }
            
            // Check only odd numbers in parallel
            let step = 2u32.to_biguint().unwrap();
            let max_candidates = 1_000_000;  // Limit for massive ranges
            
            let candidates: Vec<BigUint> = (0..max_candidates)
                .map(|i| {
                    start.clone() + &step * i.to_biguint().unwrap()
                })
                .take_while(|n| n <= range_end)
                .collect();
                
            return candidates.into_par_iter()
                .filter(|n| is_prime_biguint(n))
                .collect();
        }
    }
    
    // For smaller but still large ranges, check each odd number
    let mut start = range_start.clone();
    if &start % 2u32 == BigUint::zero() {
        start += BigUint::one();
    }
    
    let step = 2u32.to_biguint().unwrap();
    let two = 2u32.to_biguint().unwrap();
    
    let mut candidates = Vec::new();
    let mut current = start;
    
    while &current <= range_end {
        candidates.push(current.clone());
        current += &step;
    }
    
    // Special case for 2 if it's in the range
    if range_start < &two && range_end >= &two {
        primes.push(two);
    }
    
    // Test primality in parallel
    let additional_primes: Vec<BigUint> = candidates.into_par_iter()
        .filter(|n| is_prime_biguint(n))
        .collect();
    
    primes.extend(additional_primes);
    primes.sort();
    primes
}

fn check_scaled_range(m: u128, max_k: u64, max_primes_to_check: usize) {
    let start_time = Instant::now();
    let m_biguint = m.to_biguint().unwrap();
    let max_k_biguint = max_k.to_biguint().unwrap();

    // Create range boundaries
    let range_start_biguint = if m > 1 {
        (m_biguint.clone() - BigUint::one()) * 360u64
    } else {
        BigUint::one() // Start from 1 for m=1 range
    };
    let range_end_biguint = m_biguint.clone() * 360u64;

    println!(
        "\n--- Checking Primes in Range ({}, {}] (Scale m={}) ---",
        range_start_biguint, range_end_biguint, m
    );

    // --- Get Primes in the Range ---
    println!("  Generating primes in range...");
    let primes_in_range = generate_primes_in_range(&range_start_biguint, &range_end_biguint);
    
    // Limit the number of primes for very large ranges
    let primes_to_check = if primes_in_range.len() > max_primes_to_check {
        println!("  Found {} primes, limiting check to {} samples for efficiency", 
            primes_in_range.len(), max_primes_to_check);
        primes_in_range.into_iter().take(max_primes_to_check).collect()
    } else {
        primes_in_range
    };
    
    let total_primes_to_check = primes_to_check.len();
    
    if total_primes_to_check == 0 {
        println!("  No primes in this range.");
        return;
    }

    println!("  Will check {} primes in this range.", total_primes_to_check);
    
    // --- Generate Candidates ---
    
    // --- Scaled Method 1 Candidates (Factors of m * 360) ---
    println!("  Generating factors of {}...", m * 360);
    let factors_base = m_biguint.clone() * 360u64;
    let all_factors_of_base = get_factors_biguint(&factors_base);
    
    // Filter factors to only include those near the range
    let relevant_factors: Vec<BigUint> = all_factors_of_base.into_par_iter()
        .filter(|f| {
            // Include factors that might be within max_k of a prime in the range
            f >= &(range_start_biguint.clone().saturating_sub(&max_k_biguint)) &&
            f <= &(range_end_biguint.clone() + &max_k_biguint)
        })
        .collect();

    println!("  Found {} relevant factors.", relevant_factors.len());
    
    // --- Scaled Method 2 Candidates (Recursive Sequence terms) ---
    println!("  Generating sequence terms...");
    let seq_base = if m > 1 {
        (m_biguint.clone() - BigUint::one()) * 360u64 + 181u64
    } else {
        181u64.to_biguint().unwrap()
    };
    
    // Generate sequence terms within range
    let seq_terms_in_range = recursive_sequence_generator_optimized(
        &seq_base, 
        &(range_end_biguint.clone() + &max_k_biguint)
    );
    
    println!("  Generated {} sequence terms.", seq_terms_in_range.len());
    
    // --- Check Coverage in Parallel with Progress Tracking ---
    println!("  Checking proximity of primes to candidates...");
    
    let counter = Arc::new(AtomicUsize::new(0));
    let progress_interval = max(1, total_primes_to_check / 20); // Report at 5% intervals
    
    let factors_found = Arc::new(AtomicUsize::new(0));
    let seq_found = Arc::new(AtomicUsize::new(0));
    let not_found = Arc::new(AtomicUsize::new(0));
    
    let missed_primes: Vec<BigUint> = primes_to_check.par_iter()
        .filter_map(|prime| {
            let idx = counter.fetch_add(1, Ordering::Relaxed);
            
            // Show progress
            if idx % progress_interval == 0 || idx + 1 == total_primes_to_check {
                let percent = ((idx + 1) as f64 / total_primes_to_check as f64) * 100.0;
                println!("    Progress: {}/{} primes checked ({:.1}%)", 
                    idx + 1, total_primes_to_check, percent);
            }
            
            // Check Method 1 (factors)
            if check_proximity_biguint(prime, &relevant_factors, &max_k_biguint) {
                factors_found.fetch_add(1, Ordering::Relaxed);
                return None;
            }
            
            // Check Method 2 (sequence)
            if check_proximity_biguint(prime, &seq_terms_in_range, &max_k_biguint) {
                seq_found.fetch_add(1, Ordering::Relaxed);
                return None;
            }
            
            // Prime not covered by either method
            not_found.fetch_add(1, Ordering::Relaxed);
            Some(prime.clone())
        })
        .collect();

    // --- Report Results ---
    let factors_found = factors_found.load(Ordering::Relaxed);
    let seq_found = seq_found.load(Ordering::Relaxed);
    let found_count = factors_found + seq_found;
    let missed_count = not_found.load(Ordering::Relaxed);
    
    println!("  Primes in range found near Factors of {} (+/- {}): {}", 
        factors_base, max_k, factors_found);
    println!("  Primes in range found near Seq({}, +i) (+/- {}): {}", 
        seq_base, max_k, seq_found);
    println!("  Total unique primes in range found: {}", found_count);

    if missed_count == 0 {
        println!(
            "  All {} primes checked in range ({}, {}] are found by the combined scaled methods with k={}.",
            total_primes_to_check, range_start_biguint, range_end_biguint, max_k
        );
    } else {
        println!(
            "  Missed {} primes in range ({}, {}] with k={}!",
            missed_count, range_start_biguint, range_end_biguint, max_k
        );
        
        if missed_primes.len() <= 10 {
            println!("  Missed primes: {:?}", missed_primes);
        } else {
            println!("  First 10 missed primes: {:?}", missed_primes.iter().take(10).collect::<Vec<_>>());
        }
    }
    
    let duration = start_time.elapsed();
    println!("  Range check completed in: {:?}", duration);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Enhanced command line options
    let mut max_m: u128 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10); // Default to m=10
    let mut min_m: u128 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);  // Default to start at m=1
    let max_primes_to_check: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(100_000); // Cap for very large ranges
    
    if min_m > max_m {
        std::mem::swap(&mut min_m, &mut max_m);
    }

    println!("Starting prime pattern check from scale m={} to m={}", min_m, max_m);
    println!("Using MAX_K = {}", MAX_K);
    println!("Maximum primes to check per range: {}", max_primes_to_check);
    println!("Parallelism enabled with Rayon ({} threads)", rayon::current_num_threads());

    let overall_start_time = Instant::now();
    
    // Process in batches for better progress tracking with large ranges
    let mut current_m = min_m;
    
    while current_m <= max_m {
        let batch_end = min(current_m + PARALLEL_SCALE_CHUNK_SIZE as u128 - 1, max_m);
        
        println!("\nProcessing batch: m={} to m={}", current_m, batch_end);
        let batch_start_time = Instant::now();
        
        (current_m..=batch_end).into_par_iter()
            .for_each(|m| {
                check_scaled_range(m, MAX_K, max_primes_to_check);
            });
            
        current_m = batch_end + 1;
        
        let batch_duration = batch_start_time.elapsed();
        println!("\nBatch completed in: {:?}", batch_duration);
        
        // Show projected completion time for remaining batches
        if current_m <= max_m {
            let _batches_done = ((current_m - min_m) as f64) / (PARALLEL_SCALE_CHUNK_SIZE as f64);
            let batches_remaining = ((max_m - current_m + 1) as f64) / (PARALLEL_SCALE_CHUNK_SIZE as f64);
            let avg_batch_time = batch_duration.as_secs_f64() / PARALLEL_SCALE_CHUNK_SIZE as f64;
            let est_remaining = Duration::from_secs_f64(avg_batch_time * batches_remaining * PARALLEL_SCALE_CHUNK_SIZE as f64);
            
            println!("\nEstimated remaining time: {:?}", est_remaining);
        }
    }

    let overall_duration = overall_start_time.elapsed();
    println!("\nTotal execution time: {:?}", overall_duration);
}