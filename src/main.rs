use proconio::input;
use std::time::Instant;
use rand::{rngs::StdRng, SeedableRng};

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
    ops: Vec<char>,
}

impl Solver {
    fn new(seed: u64, start: Instant, input: Input) -> Self {
        let rng = StdRng::seed_from_u64(seed);

        Self {
            rng,
            start,
            input,
            ops: Vec::new(),
        }
    }

    fn solve(&mut self) {
        let path = self.build_zigzag_path();
        let start_index = path
            .iter()
            .position(|&cell| cell == (4, 0))
            .expect("initial head must be on the path");

        self.ops = path[start_index..]
            .windows(2)
            .map(|cells| Self::step(cells[0], cells[1]))
            .collect();
    }

    fn ans(&self) {
        for &op in &self.ops {
            println!("{op}");
        }
    }

    fn score(&self) -> usize {
        let mut board = self.input.f.clone();
        let mut positions = vec![(4, 0), (3, 0), (2, 0), (1, 0), (0, 0)];
        let mut colors = vec![1; 5];

        for &op in &self.ops {
            let next_head = self.advance(positions[0], op);
            let previous_tail = *positions.last().expect("snake must be non-empty");

            positions.insert(0, next_head);
            positions.pop();

            let food = board[next_head.0][next_head.1];
            if food != 0 {
                board[next_head.0][next_head.1] = 0;
                positions.push(previous_tail);
                colors.push(food);
                continue;
            }

            if let Some(h) = (1..positions.len().saturating_sub(1))
                .find(|&idx| positions[idx] == next_head)
            {
                for idx in (h + 1)..positions.len() {
                    let (i, j) = positions[idx];
                    board[i][j] = colors[idx];
                }
                positions.truncate(h + 1);
                colors.truncate(h + 1);
            }
        }

        let k = colors.len();
        let e = colors
            .iter()
            .zip(self.input.d.iter())
            .filter(|(actual, desired)| actual != desired)
            .count();

        self.ops.len() + 10000 * (e + 2 * (self.input.m - k))
    }

    fn result(&self) {
        eprintln!("{{ \"score\": {} }}", self.score());
    }

    fn build_zigzag_path(&self) -> Vec<(usize, usize)> {
        let mut path = Vec::with_capacity(self.input.n * self.input.n);

        for j in 0..self.input.n {
            if j % 2 == 0 {
                for i in 0..self.input.n {
                    path.push((i, j));
                }
            } else {
                for i in (0..self.input.n).rev() {
                    path.push((i, j));
                }
            }
        }

        path
    }

    fn step(from: (usize, usize), to: (usize, usize)) -> char {
        match (
            to.0 as isize - from.0 as isize,
            to.1 as isize - from.1 as isize,
        ) {
            (-1, 0) => 'U',
            (1, 0) => 'D',
            (0, -1) => 'L',
            (0, 1) => 'R',
            _ => panic!("path must move to an adjacent cell"),
        }
    }

    fn advance(&self, from: (usize, usize), op: char) -> (usize, usize) {
        let (di, dj) = match op {
            'U' => (-1, 0),
            'D' => (1, 0),
            'L' => (0, -1),
            'R' => (0, 1),
            _ => panic!("unknown operation"),
        };
        let ni = from.0.checked_add_signed(di).expect("move must stay on board");
        let nj = from.1.checked_add_signed(dj).expect("move must stay on board");
        assert!(ni < self.input.n && nj < self.input.n, "move must stay on board");
        (ni, nj)
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
