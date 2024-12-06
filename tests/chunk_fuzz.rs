use reggy::Search;

#[test]
fn chunk_invariant_fuzz() {
    use rand::prelude::*;

    let mut rng = SmallRng::seed_from_u64(1);

    const MIN_HAYSTACK_LEN: usize = 80;
    const MIN_PATTERN_LEN: usize = 4;
    const N_PATTERNS: usize = 4;
    const N_PARTITIONS: usize = 5;
    const N_PARTITION_RUNS: usize = 5;

    fn random_pattern(rng: &mut SmallRng) -> String {
        let d: [u8; MIN_PATTERN_LEN] = rng.gen();
        let mut res = String::with_capacity(MIN_PATTERN_LEN);
        for i in 0..MIN_PATTERN_LEN {
            res.push(((d[i] % 3) + 97) as u8 as char);
            if rng.gen::<i32>() % 2 == 0 {
                res.push('?');
            }
        }

        res
    }

    let patterns: Vec<_> = (0..N_PATTERNS).map(|_| random_pattern(&mut rng)).collect();
    println!("Patterns: {:?}", patterns);

    fn random_haystack(rng: &mut SmallRng) -> String {
        let mut res = String::with_capacity(MIN_HAYSTACK_LEN);
        for _ in 0..MIN_HAYSTACK_LEN {
            let d: u8 = rng.gen();
            res.push(((d % 3) + 97) as u8 as char);
            if rng.gen::<i32>() % 2 == 0 {
                res.push(' ');
            }
        }

        res
    }

    let haystack = random_haystack(&mut rng);
    println!("Haystack: {:?}\n", haystack);

    let mut s = Search::compile(&patterns).unwrap();

    let mut all_match_sets = vec![];

    for i in 0..N_PARTITION_RUNS {
        s.reset();
        let mut matches = vec![];

        let d: [usize; N_PARTITIONS] = rng.gen();
        let mut d: Vec<_> = d.iter().map(|i| i % haystack.len()).collect();
        d.sort();
        println!("Partition {}: {:?}", i, d);

        let mut hay_i = 0;
        for partition in d {
            if partition > hay_i {
                matches.extend(s.next(&haystack[hay_i..partition]));
                hay_i = partition;
            }
        }

        matches.extend(s.next(&haystack[hay_i..haystack.len()]));
        matches.extend(s.finish());

        all_match_sets.push(matches);
    }

    s.reset();
    let mut canonical_matches = vec![];
    canonical_matches.extend(s.next(haystack));
    canonical_matches.extend(s.finish());

    for i in 0..N_PARTITION_RUNS {
        assert_eq!(
            all_match_sets[i], canonical_matches,
            "Partition {} did not match canon",
            i
        );
    }
}
