use proconio::input;
use std::time::Instant;
use std::collections::VecDeque;
use rand::{rngs::StdRng, SeedableRng};
#[cfg(debug_assertions)]
use std::io::Write;

const FALLBACK_STALL_LIMIT: usize = 3;
const SAFE_COLLECT_TIME_LIMIT_MS: u128 = 1900;
const SAFE_COLLECT_TURN_LIMIT: usize = 10000;
const TARGET_BFS_ORDERS: [[char; 4]; 4] = [
    ['U', 'D', 'L', 'R'],
    ['R', 'D', 'L', 'U'],
    ['L', 'U', 'R', 'D'],
    ['D', 'R', 'U', 'L'],
];

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
    best_snapshot: Option<OutputSnapshot>,
    last_progress: Option<ProgressSnapshot>,
    safe_collect_active: bool,
    force_safe_collect_mode: bool,
    escape_bite_count: usize,
    rebuild_bite_count: usize,
    safe_collect_count: usize,
    previous_phase: Option<Phase>,
}

struct SnakeState {
    board: Vec<Vec<usize>>,
    positions: Vec<(usize, usize)>,
    colors: Vec<usize>,
}

#[derive(Clone, Debug)]
struct OutputSnapshot {
    ops: Vec<char>,
    score: usize,
    stats: OutputStats,
}

#[derive(Clone, Debug, Default)]
struct OutputStats {
    escape_bite_count: usize,
    rebuild_bite_count: usize,
    safe_collect_count: usize,
    forced_safe_collect: bool,
    used_safe_branch: bool,
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

impl Phase {
    fn as_str(self) -> &'static str {
        match self {
            Phase::GreedyTarget => "GreedyTarget",
            Phase::GreedyFallback => "GreedyFallback",
            Phase::SafeCollect => "SafeCollect",
            Phase::BiteRebuild => "BiteRebuild",
        }
    }
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
            best_snapshot: None,
            last_progress: None,
            safe_collect_active: false,
            force_safe_collect_mode: false,
            escape_bite_count: 0,
            rebuild_bite_count: 0,
            safe_collect_count: 0,
            previous_phase: None,
        }
    }

    fn solve(&mut self) {
        self.ops.clear();
        self.best_snapshot = None;
        let mut state = self.initial_state();
        self.last_progress = None;
        self.safe_collect_active = false;
        self.force_safe_collect_mode = false;
        self.escape_bite_count = 0;
        self.rebuild_bite_count = 0;
        self.safe_collect_count = 0;
        self.previous_phase = None;
        self.reset_phase_trace();

        while !self.should_stop(&state) && self.ops.len() < 100000 {
            let phase = self.choose_phase(&state);
            self.write_phase_trace(self.ops.len(), phase);
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
                self.run_safe_branch(&state);
            }

            if plan.phase == Phase::SafeCollect && self.previous_phase != Some(Phase::SafeCollect) {
                self.safe_collect_count += 1;
            }

            let length_before = state.colors.len();
            self.apply_moves(&mut state, &plan.moves);
            if state.colors.len() < length_before {
                if plan.phase == Phase::BiteRebuild {
                    self.rebuild_bite_count += 1;
                } else if plan.phase == Phase::SafeCollect {
                    self.escape_bite_count += 1;
                }
            }
            self.ops.extend(plan.moves.iter().copied());
            self.update_progress(&state, plan.phase, plan.resume_greedy_after_apply);
            self.previous_phase = Some(plan.phase);
        }

        let final_ops = self.ops.clone();
        self.update_best_snapshot(&final_ops, self.current_output_stats(false));
        if let Some(best) = &self.best_snapshot {
            self.ops = best.ops.clone();
        }
    }

    fn ans(&self) {
        for &op in &self.ops {
            println!("{op}");
        }
    }

    fn score(&self) -> usize {
        let state = self.simulate();
        self.absolute_score(&state, self.ops.len())
    }

    fn result(&self) {
        let state = self.simulate();
        let k = state.colors.len();
        let m = self.input.m;
        let e = state
            .colors
            .iter()
            .zip(self.input.d.iter())
            .filter(|(actual, desired)| actual != desired)
            .count();
        let t = self.ops.len();
        let prefix_len = self.prefix_len(&state);
        let remaining_food = self.remaining_food_count(&state);
        let score = self.absolute_score(&state, t);
        let stats = self
            .best_snapshot
            .as_ref()
            .map(|snapshot| snapshot.stats.clone())
            .unwrap_or_else(|| self.current_output_stats(false));

        eprintln!(
            "{{ \"score\": {}, \"k\": {}, \"m\": {}, \"e\": {}, \"t\": {}, \"prefix_len\": {}, \"remaining_food\": {}, \"completed\": {}, \"full_length\": {}, \"escape_bite_count\": {}, \"rebuild_bite_count\": {}, \"safe_collect_count\": {}, \"forced_safe_collect\": {}, \"used_safe_branch\": {}, \"elapsed_ms\": {} }}",
            score,
            k,
            m,
            e,
            t,
            prefix_len,
            remaining_food,
            k == m && e == 0,
            k == m,
            stats.escape_bite_count,
            stats.rebuild_bite_count,
            stats.safe_collect_count,
            stats.forced_safe_collect,
            stats.used_safe_branch,
            self.start.elapsed().as_millis(),
        );
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
        self.simulate_ops(&self.ops)
    }

    fn simulate_ops(&self, ops: &[char]) -> SnakeState {
        let mut state = self.initial_state();

        self.apply_moves(&mut state, ops);

        state
    }

    fn score_ops(&self, ops: &[char]) -> usize {
        let state = self.simulate_ops(ops);
        self.absolute_score(&state, ops.len())
    }

    fn absolute_score(&self, state: &SnakeState, turn_count: usize) -> usize {
        let k = state.colors.len();
        let e = state
            .colors
            .iter()
            .zip(self.input.d.iter())
            .filter(|(actual, desired)| actual != desired)
            .count();

        turn_count + 10000 * (e + 2 * (self.input.m - k))
    }

    fn current_output_stats(&self, used_safe_branch: bool) -> OutputStats {
        OutputStats {
            escape_bite_count: self.escape_bite_count,
            rebuild_bite_count: self.rebuild_bite_count,
            safe_collect_count: self.safe_collect_count,
            forced_safe_collect: self.force_safe_collect_mode,
            used_safe_branch,
        }
    }

    fn update_best_snapshot(&mut self, ops: &[char], stats: OutputStats) {
        let candidate = OutputSnapshot {
            ops: ops.to_vec(),
            score: self.score_ops(ops),
            stats,
        };
        if self
            .best_snapshot
            .as_ref()
            .is_none_or(|best| {
                candidate.score < best.score
                    || (candidate.score == best.score && candidate.ops.len() < best.ops.len())
            })
        {
            self.best_snapshot = Some(candidate);
        }
    }

    fn run_safe_branch(&mut self, state: &SnakeState) {
        let Some((branch_ops, branch_stats)) = self.build_safe_branch_ops(state) else {
            return;
        };
        self.update_best_snapshot(&branch_ops, branch_stats);
    }

    fn build_safe_branch_ops(&self, state: &SnakeState) -> Option<(Vec<char>, OutputStats)> {
        let mut branch_state = SnakeState {
            board: state.board.clone(),
            positions: state.positions.clone(),
            colors: state.colors.clone(),
        };
        let mut branch_ops = self.ops.clone();
        let mut branch_escape_bite_count = 0;

        while branch_state.colors.len() < self.input.m && branch_ops.len() < 100000 {
            let moves = if let Some(moves) = self.plan_zigzag_safe_collect_moves(&branch_state) {
                moves
            } else if let Some(moves) = self.plan_forced_safe_collect_bite(&branch_state) {
                branch_escape_bite_count += 1;
                moves
            } else {
                return None;
            };
            if moves.is_empty() {
                return None;
            }

            self.apply_moves(&mut branch_state, &moves);
            branch_ops.extend(moves);
        }

        if branch_state.colors.len() == self.input.m {
            Some((
                branch_ops,
                OutputStats {
                    escape_bite_count: branch_escape_bite_count,
                    rebuild_bite_count: 0,
                    safe_collect_count: 1,
                    forced_safe_collect: false,
                    used_safe_branch: true,
                },
            ))
        } else {
            None
        }
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
            return Phase::SafeCollect;
        }
        if state.colors.len() == self.input.m && self.prefix_len(state) < self.input.m {
            return Phase::BiteRebuild;
        }
        if self.safe_collect_active {
            self.safe_collect_active = true;
            return Phase::SafeCollect;
        }

        if self.plan_greedy_target_inner(state).is_some() {
            return Phase::GreedyTarget;
        }

        if self.plan_greedy_fallback_inner(state).is_some() {
            return Phase::GreedyFallback;
        }

        if !self.can_reach_any_food(state) {
            return Phase::SafeCollect;
        }
        Phase::GreedyFallback
    }

    fn plan_greedy_target(&self, state: &SnakeState) -> Option<Plan> {
        let (plan, bfs, target, target_color) = self.plan_greedy_target_inner(state)?;
        self.write_bfs_snapshot(
            self.ops.len(),
            Phase::GreedyTarget,
            Some(target_color),
            target.cell,
            &bfs,
        );
        Some(plan)
    }

    fn plan_greedy_fallback(&self, state: &SnakeState) -> Option<Plan> {
        let (plan, bfs, target) = self.plan_greedy_fallback_inner(state)?;
        self.write_bfs_snapshot(
            self.ops.len(),
            Phase::GreedyFallback,
            None,
            target.cell,
            &bfs,
        );
        Some(plan)
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
                .or_else(|| self.plan_greedy_fallback_inner(state).map(|(plan, _, _)| plan.moves))?;
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
        for idx in 2..state.positions.len().saturating_sub(1) {
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
            .plan_greedy_target_inner(&resumed).map(|(plan, _, _, _)| plan)
            .or_else(|| self.plan_greedy_fallback_inner(&resumed).map(|(plan, _, _)| plan))
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

    fn plan_greedy_target_inner(
        &self,
        state: &SnakeState,
    ) -> Option<(Plan, BfsResult, FoodTarget, usize)> {
        let target_color = self.input.d[state.colors.len()];
        let primary_bfs =
            self.bfs_reachable_target_color_with_order(state, target_color, &TARGET_BFS_ORDERS[0]);
        let target = self.choose_nearest_food_of_color(state, &primary_bfs, target_color)?;
        let mut seen_moves = Vec::new();
        let mut best: Option<(bool, usize, bool, usize, usize, usize, Plan, BfsResult)> = None;

        for order in TARGET_BFS_ORDERS {
            let bfs = self.bfs_reachable_target_color_with_order(state, target_color, &order);
            let Some(moves) = bfs.restore_moves(target.cell) else {
                continue;
            };
            if seen_moves.iter().any(|existing: &Vec<char>| *existing == moves) {
                continue;
            }
            seen_moves.push(moves.clone());

            let (next_target_dist, next_fallback_dist, next_prefix_len) =
                self.evaluate_greedy_target_candidate(state, &moves);
            let candidate = (
                next_target_dist.is_some(),
                next_target_dist.unwrap_or(usize::MAX),
                next_fallback_dist.is_some(),
                next_fallback_dist.unwrap_or(usize::MAX),
                next_prefix_len,
                moves.len(),
                Plan {
                    phase: Phase::GreedyTarget,
                    moves,
                    resume_greedy_after_apply: false,
                },
                bfs,
            );

            if best.as_ref().is_none_or(|current| {
                candidate.0 && !current.0
                    || (candidate.0 == current.0 && candidate.1 < current.1)
                    || (candidate.0 == current.0
                        && candidate.1 == current.1
                        && candidate.2
                        && !current.2)
                    || (candidate.0 == current.0
                        && candidate.1 == current.1
                        && candidate.2 == current.2
                        && candidate.3 < current.3)
                    || (candidate.0 == current.0
                        && candidate.1 == current.1
                        && candidate.2 == current.2
                        && candidate.3 == current.3
                        && candidate.4 > current.4)
                    || (candidate.0 == current.0
                        && candidate.1 == current.1
                        && candidate.2 == current.2
                        && candidate.3 == current.3
                        && candidate.4 == current.4
                        && candidate.5 < current.5)
            }) {
                best = Some(candidate);
            }
        }

        let (_, _, _, _, _, _, plan, bfs) = best?;
        Some((plan, bfs, target, target_color))
    }

    fn evaluate_greedy_target_candidate(
        &self,
        state: &SnakeState,
        moves: &[char],
    ) -> (Option<usize>, Option<usize>, usize) {
        let mut next_state = SnakeState {
            board: state.board.clone(),
            positions: state.positions.clone(),
            colors: state.colors.clone(),
        };
        self.apply_moves(&mut next_state, moves);

        let next_target_dist = if next_state.colors.len() < self.input.m {
            let next_target_color = self.input.d[next_state.colors.len()];
            let next_bfs = self.bfs_reachable_target_color(&next_state, next_target_color);
            self.choose_nearest_food_of_color(&next_state, &next_bfs, next_target_color)
                .map(|target| target.dist)
        } else {
            Some(0)
        };
        let next_fallback_dist = if next_state.colors.len() < self.input.m {
            let next_bfs = self.bfs_reachable_first_food(&next_state);
            self.choose_nearest_food(&next_state, &next_bfs)
                .map(|target| target.dist)
        } else {
            Some(0)
        };

        (next_target_dist, next_fallback_dist, self.prefix_len(&next_state))
    }

    fn plan_greedy_fallback_inner(
        &self,
        state: &SnakeState,
    ) -> Option<(Plan, BfsResult, FoodTarget)> {
        let bfs = self.bfs_reachable_first_food(state);
        let target = self.choose_nearest_food(state, &bfs)?;
        let moves = bfs.restore_moves(target.cell)?;
        let plan = Plan {
            phase: Phase::GreedyFallback,
            moves,
            resume_greedy_after_apply: false,
        };
        Some((plan, bfs, target))
    }

    #[cfg(debug_assertions)]
    fn reset_phase_trace(&self) {
        let _ = std::fs::File::create("debug.txt");
    }

    #[cfg(not(debug_assertions))]
    fn reset_phase_trace(&self) {}

    #[cfg(debug_assertions)]
    fn write_phase_trace(&self, turn: usize, phase: Phase) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("debug.txt")
        {
            let _ = writeln!(file, "{} {}", turn, phase.as_str());
        }
    }

    #[cfg(not(debug_assertions))]
    fn write_phase_trace(&self, _turn: usize, _phase: Phase) {}

    #[cfg(debug_assertions)]
    fn write_bfs_snapshot(
        &self,
        turn: usize,
        phase: Phase,
        target_color: Option<usize>,
        target: (usize, usize),
        bfs: &BfsResult,
    ) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("debug.txt")
        {
            let _ = writeln!(file, "TURN {}", turn);
            let _ = writeln!(file, "PHASE {}", phase.as_str());
            if let Some(target_color) = target_color {
                let _ = writeln!(file, "TARGET_COLOR {}", target_color);
            }
            let _ = writeln!(file, "TARGET {} {}", target.0, target.1);
            let _ = writeln!(file, "BFS");
            let _ = write!(file, "{}", bfs.dist_debug_text());
            let _ = writeln!(file, "END");
        }
    }

    #[cfg(not(debug_assertions))]
    fn write_bfs_snapshot(
        &self,
        _turn: usize,
        _phase: Phase,
        _target_color: Option<usize>,
        _target: (usize, usize),
        _bfs: &BfsResult,
    ) {
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
        let cell_open_turn = self.build_cell_open_turn(state);
        let mut queue = VecDeque::new();

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
                let arrival_turn = current_dist + 1;
                let still_occupied = cell_open_turn[next.0][next.1] > arrival_turn;
                if still_occupied || result.reachable[next.0][next.1] {
                    continue;
                }

                result.reachable[next.0][next.1] = true;
                result.dist[next.0][next.1] = Some(arrival_turn);
                result.parent[next.0][next.1] = Some(current);
                result.parent_move[next.0][next.1] = Some(op);
                queue.push_back(next);
            }
        }

        result
    }

    fn bfs_reachable_target_color(&self, state: &SnakeState, target_color: usize) -> BfsResult {
        self.bfs_reachable_target_color_with_order(state, target_color, &TARGET_BFS_ORDERS[0])
    }

    fn bfs_reachable_target_color_with_order(
        &self,
        state: &SnakeState,
        target_color: usize,
        order: &[char; 4],
    ) -> BfsResult {
        let start = state.positions[0];
        let mut result = BfsResult::new(self.input.n, start);
        let mut cell_open_turn = self.build_cell_open_turn(state);
        let mut queue = VecDeque::new();
        for i in 0..self.input.n {
            for j in 0..self.input.n {
                let food = state.board[i][j];
                if food != 0 && food != target_color {
                    cell_open_turn[i][j] = usize::MAX;
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

            for &op in order {
                let Some(next) = self.try_advance(current, op) else {
                    continue;
                };
                let arrival_turn = current_dist + 1;
                let still_occupied = cell_open_turn[next.0][next.1] > arrival_turn;
                if still_occupied || result.reachable[next.0][next.1] {
                    continue;
                }

                result.reachable[next.0][next.1] = true;
                result.dist[next.0][next.1] = Some(arrival_turn);
                result.parent[next.0][next.1] = Some(current);
                result.parent_move[next.0][next.1] = Some(op);
                queue.push_back(next);
            }
        }

        result
    }

    fn build_cell_open_turn(&self, state: &SnakeState) -> Vec<Vec<usize>> {
        let mut cell_open_turn = vec![vec![0; self.input.n]; self.input.n];
        for (l, &(i, j)) in state.positions.iter().enumerate().skip(1).rev() {
            cell_open_turn[i][j] = state.colors.len() - l - 1;
        }
        cell_open_turn
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

    fn dist_debug_text(&self) -> String {
        let mut lines = Vec::with_capacity(self.dist.len());
        for row in &self.dist {
            let line = row
                .iter()
                .map(|cell| match cell {
                    Some(dist) => dist.to_string(),
                    None => ".".to_owned(),
                })
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(line);
        }
        let mut text = lines.join("\n");
        text.push('\n');
        text
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
