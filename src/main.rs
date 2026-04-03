use proconio::input;
use std::time::Instant;
use std::collections::VecDeque;
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

struct SnakeState {
    board: Vec<Vec<usize>>,
    positions: Vec<(usize, usize)>,
    colors: Vec<usize>,
}

#[derive(Clone, Debug)]
struct BfsResult {
    start: (usize, usize),
    reachable: Vec<Vec<bool>>,
    dist: Vec<Vec<Option<usize>>>,
    parent: Vec<Vec<Option<(usize, usize)>>>,
    parent_move: Vec<Vec<Option<char>>>,
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
        let state = self.simulate();
        let k = state.colors.len();
        let e = state.colors
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

    fn simulate(&self) -> SnakeState {
        let mut state = SnakeState {
            board: self.input.f.clone(),
            positions: vec![(4, 0), (3, 0), (2, 0), (1, 0), (0, 0)],
            colors: vec![1; 5],
        };

        for &op in &self.ops {
            self.apply_move(&mut state, op);
        }

        state
    }

    fn bfs_reachable_cells(&self, state: &SnakeState) -> BfsResult {
        let start = state.positions[0];
        let mut result = BfsResult::new(self.input.n, start);
        let mut blocked = vec![vec![false; self.input.n]; self.input.n];
        let mut queue = VecDeque::new();

        // 噛み切りなしの初期版では、現在の頭以外の蛇マスを障害物とみなす。
        for &(i, j) in state.positions.iter().skip(1) {
            blocked[i][j] = true;
        }

        result.reachable[start.0][start.1] = true;
        result.dist[start.0][start.1] = Some(0);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            let current_dist =
                result.dist[current.0][current.1].expect("visited cell must have distance");

            for op in ['U', 'D', 'L', 'R'] {
                let Some(next) = self.try_advance(current, op) else {
                    continue;
                };
                if blocked[next.0][next.1] || result.reachable[next.0][next.1] {
                    continue;
                }

                result.reachable[next.0][next.1] = true;
                result.dist[next.0][next.1] = Some(current_dist + 1);
                result.parent[next.0][next.1] = Some(current);
                result.parent_move[next.0][next.1] = Some(op);
                queue.push_back(next);
            }
        }

        result
    }

    fn apply_move(&self, state: &mut SnakeState, op: char) {
        let next_head = self.advance(state.positions[0], op);
        let previous_tail = *state.positions.last().expect("snake must be non-empty");

        state.positions.insert(0, next_head);
        state.positions.pop();

        let food = state.board[next_head.0][next_head.1];
        if food != 0 {
            state.board[next_head.0][next_head.1] = 0;
            state.positions.push(previous_tail);
            state.colors.push(food);
            return;
        }

        if let Some(h) = (1..state.positions.len().saturating_sub(1))
            .find(|&idx| state.positions[idx] == next_head)
        {
            for idx in (h + 1)..state.positions.len() {
                let (i, j) = state.positions[idx];
                state.board[i][j] = state.colors[idx];
            }
            state.positions.truncate(h + 1);
            state.colors.truncate(h + 1);
        }
    }

    fn advance(&self, from: (usize, usize), op: char) -> (usize, usize) {
        self.try_advance(from, op).expect("move must stay on board")
    }

    fn try_advance(&self, from: (usize, usize), op: char) -> Option<(usize, usize)> {
        let (di, dj) = match op {
            'U' => (-1, 0),
            'D' => (1, 0),
            'L' => (0, -1),
            'R' => (0, 1),
            _ => panic!("unknown operation"),
        };
        let ni = from.0.checked_add_signed(di)?;
        let nj = from.1.checked_add_signed(dj)?;
        if ni >= self.input.n || nj >= self.input.n {
            return None;
        }
        Some((ni, nj))
    }
}

impl BfsResult {
    fn new(n: usize, start: (usize, usize)) -> Self {
        Self {
            start,
            reachable: vec![vec![false; n]; n],
            dist: vec![vec![None; n]; n],
            parent: vec![vec![None; n]; n],
            parent_move: vec![vec![None; n]; n],
        }
    }

    fn can_reach(&self, cell: (usize, usize)) -> bool {
        self.reachable[cell.0][cell.1]
    }

    fn distance(&self, cell: (usize, usize)) -> Option<usize> {
        self.dist[cell.0][cell.1]
    }

    fn parent_of(&self, cell: (usize, usize)) -> Option<(usize, usize)> {
        self.parent[cell.0][cell.1]
    }

    fn restore_moves(&self, goal: (usize, usize)) -> Option<Vec<char>> {
        if !self.can_reach(goal) {
            return None;
        }

        let mut moves = Vec::new();
        let mut current = goal;
        while current != self.start {
            moves.push(self.parent_move[current.0][current.1]?);
            current = self.parent[current.0][current.1]?;
        }
        moves.reverse();
        Some(moves)
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
