use proconio::input;
use std::time::Instant;
use rand::{rngs::StdRng, Rng, SeedableRng};

fn read_input() -> Input {
    input! {
        n: usize,
        m: usize,
        c: usize,
        d: [usize; m],
        f: [[usize; n]; n],
    }

    Input { n, m, c, d, f }
}

#[derive(Clone, Debug)]
struct Input {
    n: usize,
    m: usize,
    c: usize,
    d: Vec<usize>,
    f: Vec<Vec<usize>>,
}

struct Solver {
    rng: StdRng,
    start: Instant,
    input: Input,
}

impl Solver {
    fn new(seed: u64, start: Instant, input: Input) -> Self {
        let rng = StdRng::seed_from_u64(seed);

        Self {
            rng,
            start,
            input,
        }
    }

    fn solve(&mut self) {
        // ここに解法を実装
    }

    fn ans(&self) {
        // ここに解答の出力を実装
    }

    fn score(&self) -> usize {
        self.input.n
    }

    fn result(&self) {
        eprintln!("{{ \"score\": {} }}", self.score());
    }
}

fn main() {
    let start = Instant::now();
    let input = read_input();
    let mut solver = Solver::new(0, start, input);

    solver.solve();
    solver.ans();
    solver.result();
}
