use proconio::input;
use std::time::Instant;
use std::collections::VecDeque;
use rand::{rngs::StdRng, SeedableRng};

const FALLBACK_STALL_LIMIT: usize = 3;
const SAFE_COLLECT_TIME_LIMIT_MS: u128 = 1900;
const SAFE_COLLECT_TURN_LIMIT: usize = 10000;

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
    last_progress: Option<ProgressSnapshot>,
    safe_collect_active: bool,
    force_safe_collect_mode: bool,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FoodTarget {
    cell: (usize, usize),
    dist: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    GreedyTarget,
    GreedyFallback,
    SafeCollect,
    BiteRebuild,
}

#[derive(Clone, Debug)]
struct Plan {
    phase: Phase,
    moves: Vec<char>,
    resume_greedy_after_apply: bool,
}

#[derive(Clone, Copy, Debug)]
struct ProgressSnapshot {
    prefix_len: usize,
    remaining_food_count: usize,
    can_reach_any_food: bool,
    fallback_stall_count: usize,
    is_stalled: bool,
}

impl Solver {
    fn new(seed: u64, start: Instant, input: Input) -> Self {
        let rng = StdRng::seed_from_u64(seed);

        Self {
            rng,
            start,
            input,
            ops: Vec::new(),
            last_progress: None,
            safe_collect_active: false,
            force_safe_collect_mode: false,
        }
    }

    fn solve(&mut self) {
        self.ops.clear();
        let mut state = self.initial_state();
        self.last_progress = None;
        self.safe_collect_active = false;
        self.force_safe_collect_mode = false;

        while !self.should_stop(&state) && self.ops.len() < 100000 {
            let phase = self.choose_phase(&state);
            let plan = match phase {
                Phase::GreedyTarget => self.plan_greedy_target(&state),
                Phase::GreedyFallback => self.plan_greedy_fallback(&state),
                Phase::SafeCollect => self.plan_safe_collect(&state),
                Phase::BiteRebuild => self.plan_bite_rebuild(&state),
            };
            let Some(plan) = plan else {
                break;
            };

            if plan.moves.is_empty() {
                break;
            }

            if self.should_launch_safe_branch(&state, &plan) {
                // safe branch execution will be added in the next TODO.
            }

            self.apply_moves(&mut state, &plan.moves);
            self.ops.extend(plan.moves.iter().copied());
            self.update_progress(&state, plan.phase, plan.resume_greedy_after_apply);
        }
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

    fn initial_state(&self) -> SnakeState {
        SnakeState {
            board: self.input.f.clone(),
            positions: vec![(4, 0), (3, 0), (2, 0), (1, 0), (0, 0)],
            colors: vec![1; 5],
        }
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
        let mut state = self.initial_state();

        self.apply_moves(&mut state, &self.ops);

        state
    }

    fn should_stop(&self, state: &SnakeState) -> bool {
        if self.force_safe_collect_mode && state.colors.len() == self.input.m {
            return true;
        }
        state.colors.len() == self.input.m && self.prefix_len(state) == self.input.m
    }

    fn choose_phase(&mut self, state: &SnakeState) -> Phase {
        if self.should_force_safe_collect() {
            self.force_safe_collect_mode = true;
            self.safe_collect_active = true;
        }
        if self.force_safe_collect_mode {
            return Phase::SafeCollect;
        }
        if state.colors.len() == self.input.m && self.prefix_len(state) < self.input.m {
            return Phase::BiteRebuild;
        }
        if self.safe_collect_active {
            self.safe_collect_active = true;
            return Phase::SafeCollect;
        }

        if self.plan_greedy_target(state).is_some() {
            return Phase::GreedyTarget;
        }

        if self.plan_greedy_fallback(state).is_some() {
            return Phase::GreedyFallback;
        }

        if !self.can_reach_any_food(state) {
            return Phase::SafeCollect;
        }
        Phase::GreedyFallback
    }

    fn plan_greedy_target(&self, state: &SnakeState) -> Option<Plan> {
        let target_color = self.input.d[state.colors.len()];
        let bfs = self.bfs_reachable_target_color(state, target_color);
        let target = self.choose_nearest_food_of_color(state, &bfs, target_color)?;
        let moves = bfs.restore_moves(target.cell)?;
        Some(Plan {
            phase: Phase::GreedyTarget,
            moves,
            resume_greedy_after_apply: false,
        })
    }

    fn plan_greedy_fallback(&self, state: &SnakeState) -> Option<Plan> {
        let bfs = self.bfs_reachable_first_food(state);
        let target = self.choose_nearest_food(state, &bfs)?;
        let moves = bfs.restore_moves(target.cell)?;
        Some(Plan {
            phase: Phase::GreedyFallback,
            moves,
            resume_greedy_after_apply: false,
        })
    }

    fn plan_safe_collect(&self, state: &SnakeState) -> Option<Plan> {
        let (moves, resume_greedy_after_apply) = if self.force_safe_collect_mode {
            let moves = self
                .plan_zigzag_safe_collect_moves(state)
                .or_else(|| self.plan_forced_safe_collect_bite(state))?;
            (moves, false)
        } else {
            let bite_moves = self.plan_safe_collect_bfs_bite(state);
            let resume_greedy_after_apply = bite_moves.is_some();
            let moves = bite_moves
                .or_else(|| self.plan_zigzag_safe_collect_moves(state))
                .or_else(|| self.plan_greedy_fallback(state).map(|plan| plan.moves))?;
            (moves, resume_greedy_after_apply)
        };
        Some(Plan {
            phase: Phase::SafeCollect,
            moves,
            resume_greedy_after_apply,
        })
    }

    fn plan_bite_rebuild(&self, state: &SnakeState) -> Option<Plan> {
        let mut best: Option<(usize, usize, usize, Vec<char>)> = None;

        for idx in 2..state.positions.len().saturating_sub(1) {
            let Some((moves, bitten)) = self.simulate_bite_candidate(state, idx) else {
                continue;
            };
            let prefix_len = self.prefix_len(&bitten);
            if prefix_len != bitten.colors.len() {
                continue;
            }
            let repaired_prefix_len = self.project_prefix_after_resume(&bitten);
            let candidate = (repaired_prefix_len, prefix_len, bitten.colors.len(), moves);
            if best.as_ref().is_none_or(|current| {
                candidate.0 > current.0
                    || (candidate.0 == current.0 && candidate.1 > current.1)
                    || (candidate.0 == current.0 && candidate.1 == current.1 && candidate.2 > current.2)
                    || (candidate.0 == current.0
                        && candidate.1 == current.1
                        && candidate.2 == current.2
                        && candidate.3.len() < current.3.len())
            }) {
                best = Some(candidate);
            }
        }

        let (_, _, _, moves) = best?;
        Some(Plan {
            phase: Phase::BiteRebuild,
            moves,
            resume_greedy_after_apply: false,
        })
    }

    fn apply_moves(&self, state: &mut SnakeState, moves: &[char]) {
        for &op in moves {
            self.apply_move(state, op);
        }
    }

    fn plan_safe_collect_bfs_bite(&self, state: &SnakeState) -> Option<Vec<char>> {
        if self.can_reach_any_food(state) {
            return None;
        }

        self.plan_forced_safe_collect_bite(state)
    }

    fn plan_forced_safe_collect_bite(&self, state: &SnakeState) -> Option<Vec<char>> {
        for idx in (2..state.positions.len().saturating_sub(1)).rev() {
            if let Some((moves, _)) = self.simulate_bite_candidate(state, idx) {
                return Some(moves);
            }
        }

        None
    }

    fn simulate_bite_candidate(
        &self,
        state: &SnakeState,
        idx: usize,
    ) -> Option<(Vec<char>, SnakeState)> {
        let target = *state.positions.get(idx)?;
        let bfs = self.bfs_reachable_body_target(state, target);
        let moves = bfs.restore_moves(target)?;
        let mut bitten = SnakeState {
            board: state.board.clone(),
            positions: state.positions.clone(),
            colors: state.colors.clone(),
        };
        self.apply_moves(&mut bitten, &moves);
        if bitten.colors.len() >= state.colors.len() {
            return None;
        }
        Some((moves, bitten))
    }

    fn project_prefix_after_resume(&self, state: &SnakeState) -> usize {
        let mut resumed = SnakeState {
            board: state.board.clone(),
            positions: state.positions.clone(),
            colors: state.colors.clone(),
        };

        let plan = self
            .plan_greedy_target(&resumed)
            .or_else(|| self.plan_greedy_fallback(&resumed))
            .or_else(|| self.plan_zigzag_safe_collect_moves(&resumed).map(|moves| Plan {
                phase: Phase::SafeCollect,
                moves,
                resume_greedy_after_apply: false,
            }));

        if let Some(plan) = plan {
            self.apply_moves(&mut resumed, &plan.moves);
        }

        self.prefix_len(&resumed)
    }

    fn should_force_safe_collect(&self) -> bool {
        self.start.elapsed().as_millis() >= SAFE_COLLECT_TIME_LIMIT_MS
            || self.ops.len() >= SAFE_COLLECT_TURN_LIMIT
    }

    fn should_launch_safe_branch(&self, state: &SnakeState, plan: &Plan) -> bool {
        !self.force_safe_collect_mode
            && state.colors.len() < self.input.m
            && plan.phase == Phase::SafeCollect
            && plan.resume_greedy_after_apply
    }

    fn bfs_reachable_body_target(&self, state: &SnakeState, target: (usize, usize)) -> BfsResult {
        let start = state.positions[0];
        let mut result = BfsResult::new(self.input.n, start);
        let mut blocked = vec![vec![false; self.input.n]; self.input.n];
        let mut queue = VecDeque::new();

        for &(i, j) in state.positions.iter().skip(1) {
            blocked[i][j] = true;
        }
        for i in 0..self.input.n {
            for j in 0..self.input.n {
                if state.board[i][j] != 0 {
                    blocked[i][j] = true;
                }
            }
        }
        blocked[target.0][target.1] = false;

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
                if next == target {
                    return result;
                }
                queue.push_back(next);
            }
        }

        result
    }

    fn plan_zigzag_safe_collect_moves(&self, state: &SnakeState) -> Option<Vec<char>> {
        let path = self.build_zigzag_path();
        let start = state.positions[0];
        let start_idx = path.iter().position(|&cell| cell == start)?;
        let mut blocked = vec![vec![false; self.input.n]; self.input.n];
        for &(i, j) in state.positions.iter().skip(1) {
            blocked[i][j] = true;
        }

        let forward = self.collect_along_zigzag_direction(state, &path, &blocked, start_idx, 1);
        let backward = self.collect_along_zigzag_direction(state, &path, &blocked, start_idx, -1);

        match (forward, backward) {
            (Some(left), Some(right)) => {
                if left.len() <= right.len() {
                    Some(left)
                } else {
                    Some(right)
                }
            }
            (Some(moves), None) | (None, Some(moves)) => Some(moves),
            (None, None) => None,
        }
    }

    fn collect_along_zigzag_direction(
        &self,
        state: &SnakeState,
        path: &[(usize, usize)],
        blocked: &[Vec<bool>],
        start_idx: usize,
        delta: isize,
    ) -> Option<Vec<char>> {
        let mut current = path[start_idx];
        let mut idx = start_idx as isize;
        let mut moves = Vec::new();

        loop {
            idx += delta;
            if !(0..path.len() as isize).contains(&idx) {
                return None;
            }

            let next = path[idx as usize];
            if blocked[next.0][next.1] {
                return None;
            }

            moves.push(Self::step(current, next));
            if state.board[next.0][next.1] != 0 {
                return Some(moves);
            }

            current = next;
        }
    }

    fn update_progress(
        &mut self,
        state: &SnakeState,
        phase: Phase,
        resume_greedy_after_apply: bool,
    ) {
        if self.force_safe_collect_mode {
            self.safe_collect_active = state.colors.len() < self.input.m;
        } else if phase == Phase::BiteRebuild {
            self.safe_collect_active = false;
        } else if resume_greedy_after_apply {
            self.safe_collect_active = false;
        }
        let previous = self.last_progress;
        let prefix_len = self.prefix_len(state);
        let remaining_food_count = self.remaining_food_count(state);
        let can_reach_any_food = self.can_reach_any_food(state);
        let fallback_stall_count =
            self.fallback_stall_count(previous, prefix_len, remaining_food_count, phase);

        self.last_progress = Some(ProgressSnapshot {
            prefix_len,
            remaining_food_count,
            can_reach_any_food,
            fallback_stall_count,
            is_stalled: fallback_stall_count >= FALLBACK_STALL_LIMIT,
        });
    }

    fn prefix_len(&self, state: &SnakeState) -> usize {
        state.colors
            .iter()
            .zip(self.input.d.iter())
            .take_while(|(actual, desired)| actual == desired)
            .count()
    }

    fn remaining_food_count(&self, state: &SnakeState) -> usize {
        state.board
            .iter()
            .map(|row| row.iter().filter(|&&food| food != 0).count())
            .sum()
    }

    fn can_reach_any_food(&self, state: &SnakeState) -> bool {
        let bfs = self.bfs_reachable_first_food(state);
        self.choose_nearest_food(state, &bfs).is_some()
    }

    fn fallback_stall_count(
        &self,
        previous: Option<ProgressSnapshot>,
        prefix_len: usize,
        remaining_food_count: usize,
        phase: Phase,
    ) -> usize {
        let Some(previous) = previous else {
            return 0;
        };
        if phase != Phase::GreedyFallback {
            return 0;
        }

        let prefix_grew = prefix_len > previous.prefix_len;
        let food_collected = remaining_food_count < previous.remaining_food_count;
        if food_collected && !prefix_grew {
            previous.fallback_stall_count + 1
        } else {
            0
        }
    }

    fn is_progress_stalled(&self) -> bool {
        self.last_progress
            .is_some_and(|progress| progress.is_stalled)
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

    fn bfs_reachable_first_food(&self, state: &SnakeState) -> BfsResult {
        let start = state.positions[0];
        let mut result = BfsResult::new(self.input.n, start);
        let mut blocked = vec![vec![false; self.input.n]; self.input.n];
        let mut queue = VecDeque::new();

        for &(i, j) in state.positions.iter().skip(1) {
            blocked[i][j] = true;
        }

        result.reachable[start.0][start.1] = true;
        result.dist[start.0][start.1] = Some(0);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            let current_dist =
                result.dist[current.0][current.1].expect("visited cell must have distance");

            // 任意餌 fallback でも、最初に到達した餌で一度止まって再計画する。
            if current != start && state.board[current.0][current.1] != 0 {
                continue;
            }

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

    fn bfs_reachable_target_color(&self, state: &SnakeState, target_color: usize) -> BfsResult {
        let start = state.positions[0];
        let mut result = BfsResult::new(self.input.n, start);
        let mut blocked = vec![vec![false; self.input.n]; self.input.n];
        let mut queue = VecDeque::new();

        for &(i, j) in state.positions.iter().skip(1) {
            blocked[i][j] = true;
        }
        for i in 0..self.input.n {
            for j in 0..self.input.n {
                let food = state.board[i][j];
                if food != 0 && food != target_color {
                    blocked[i][j] = true;
                }
            }
        }

        result.reachable[start.0][start.1] = true;
        result.dist[start.0][start.1] = Some(0);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            let current_dist =
                result.dist[current.0][current.1].expect("visited cell must have distance");

            // 同色餌に到達したら、そこで止まる経路だけを候補にしたいので先へは展開しない。
            if current != start && state.board[current.0][current.1] == target_color {
                continue;
            }

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

    fn choose_nearest_food_of_color(
        &self,
        state: &SnakeState,
        bfs: &BfsResult,
        target_color: usize,
    ) -> Option<FoodTarget> {
        let mut best = None;

        for i in 0..self.input.n {
            for j in 0..self.input.n {
                if state.board[i][j] != target_color {
                    continue;
                }
                let Some(dist) = bfs.distance((i, j)) else {
                    continue;
                };
                let candidate = FoodTarget { cell: (i, j), dist };
                if best.is_none_or(|current: FoodTarget| {
                    candidate.dist < current.dist
                        || (candidate.dist == current.dist && candidate.cell < current.cell)
                }) {
                    best = Some(candidate);
                }
            }
        }

        best
    }

    fn choose_nearest_food(&self, state: &SnakeState, bfs: &BfsResult) -> Option<FoodTarget> {
        let mut best = None;

        for i in 0..self.input.n {
            for j in 0..self.input.n {
                if state.board[i][j] == 0 {
                    continue;
                }
                let Some(dist) = bfs.distance((i, j)) else {
                    continue;
                };
                let candidate = FoodTarget { cell: (i, j), dist };
                if best.is_none_or(|current: FoodTarget| {
                    candidate.dist < current.dist
                        || (candidate.dist == current.dist && candidate.cell < current.cell)
                }) {
                    best = Some(candidate);
                }
            }
        }

        best
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
