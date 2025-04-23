# 360 Prime Pattern Explorer

A high-performance tool to verify a pattern in the distribution of prime numbers at massive scales. This implementation efficiently tests the "360 conjecture" about prime number locations across ranges reaching into the hundreds of millions and beyond.

## The Prime Pattern Hypothesis

This tool tests the following conjecture about prime numbers:

Every prime number P can be located within a distance of at most 180 units from at least one of these candidates:
1. The factors of m×360, where m is the segment scale factor
2. Terms in a recursive sequence starting at (m-1)×360+181

For any integer m≥1, the pattern holds for all primes in the range ((m-1)×360, m×360].

## Key Features

- **Massive Scale Testing**: Verifies the pattern for scales far beyond previous implementations
- **Parallel Processing**: Utilizes Rayon for multi-threaded prime checking and verification
- **Arbitrary Precision**: Uses BigUint for handling extremely large numbers
- **Progress Tracking**: Reports progress during long-running checks
- **Memory Efficient**: Optimized to avoid excessive memory usage at large scales
- **Sampling Mode**: For extremely large ranges, uses a representative sampling approach

## Usage

```bash
cargo run --release [max_m] [min_m] [max_primes_per_range]
```

Parameters:
- `max_m`: The maximum scale factor to test (default: 10)
- `min_m`: The minimum scale factor to test (default: 1)
- `max_primes_per_range`: The maximum number of primes to check per range (default: 100,000)

Example for testing from scale 1 million to 1 million + 10:
```bash
cargo run --release 1000010 1000000 1000
```

## Performance Considerations

- **Memory vs. Scale**: At extremely large scales (m > 10^9), memory usage for storing prime lists becomes significant
- **Sampling Mode**: Automatically activates for ranges with too many primes
- **Primality Testing**: Uses specialized algorithms for different number sizes
- **Batch Processing**: Divides large scale ranges into manageable batches

## Implementation Details

The code employs several optimization strategies:

1. **Efficient Prime Generation**:
   - Uses primal's Sieve for ranges within u64::MAX
   - Uses num_prime with parallel testing for larger ranges
   - Implements sampling for extremely large ranges

2. **Optimized Factor Generation**:
   - For numbers under u64::MAX, uses primal's efficient factorization
   - For larger numbers, uses a targeted approach focusing on relevant factors only
   - Avoids unnecessary computation of all factors for massive numbers

3. **Memory Management**:
   - Limits prime list size for very large ranges
   - Uses stream-like processing where appropriate
   - Applies parallel processing with controlled batch sizes

4. **Progress Reporting**:
   - Provides detailed progress updates for long-running operations
   - Estimates completion time for large scale tests

## Results

Initial testing confirms the pattern holds for all scales tested. The code can verify far larger scales than previous implementations, allowing for more comprehensive testing of the hypothesis.

## Requirements

- Rust 1.54 or later
- At least 4GB RAM (16+GB recommended for scales beyond 10^8)
- Multi-core CPU (8+ cores recommended for optimal performance)
